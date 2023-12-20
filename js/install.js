'use strict';

const fs = require('fs');
const http = require('http');
const os = require('os');
const path = require('path');
const crypto = require('crypto');
const zlib = require('zlib');
const stream = require('stream');
const process = require('process');

const HttpsProxyAgent = require('https-proxy-agent');
const fetch = require('node-fetch');
const ProgressBar = require('progress');
const Proxy = require('proxy-from-env');
// NOTE: Can be dropped in favor of `fs.mkdirSync(path, { recursive: true })` once we stop supporting Node 8.x
const mkdirp = require('mkdirp');
const which = require('which');

const helper = require('./helper');
const pkgInfo = require('../package.json');
const Logger = require('./logger');

function getLogStream(defaultStream) {
  const logStream = process.env.SENTRYCLI_LOG_STREAM || defaultStream;

  if (logStream === 'stdout') {
    return process.stdout;
  }

  if (logStream === 'stderr') {
    return process.stderr;
  }

  throw new Error(
    `Incorrect SENTRYCLI_LOG_STREAM env variable. Possible values: 'stdout' | 'stderr'`
  );
}

const ttyLogger = new Logger(getLogStream('stderr'));

const CDN_URL =
  process.env.SENTRYCLI_LOCAL_CDNURL ||
  process.env.npm_config_sentrycli_cdnurl ||
  process.env.SENTRYCLI_CDNURL ||
  'https://downloads.sentry-cdn.com/sentry-cli';

function shouldRenderProgressBar() {
  const silentFlag = process.argv.some((v) => v === '--silent');
  const silentConfig = process.env.npm_config_loglevel === 'silent';
  // Leave `SENTRY_NO_PROGRESS_BAR` for backwards compatibility
  const silentEnv = process.env.SENTRYCLI_NO_PROGRESS_BAR || process.env.SENTRY_NO_PROGRESS_BAR;
  const ciEnv = process.env.CI === 'true';
  // If any of possible options is set, skip rendering of progress bar
  return !(silentFlag || silentConfig || silentEnv || ciEnv);
}

function getDownloadUrl(platform, arch) {
  const releasesUrl = `${CDN_URL}/${pkgInfo.version}/sentry-cli`;
  let archString = '';
  switch (arch) {
    case 'x64':
      archString = 'x86_64';
      break;
    case 'x86':
    case 'ia32':
      archString = 'i686';
      break;
    case 'arm64':
      archString = 'aarch64';
      break;
    case 'arm':
      archString = 'armv7';
      break;
    default:
      archString = arch;
  }
  switch (platform) {
    case 'darwin':
      return `${releasesUrl}-Darwin-universal`;
    case 'win32':
      return `${releasesUrl}-Windows-${archString}.exe`;
    case 'linux':
    case 'freebsd':
      return `${releasesUrl}-Linux-${archString}`;
    default:
      return null;
  }
}

function createProgressBar(name, total) {
  const incorrectTotal = typeof total !== 'number' || Number.isNaN(total);

  if (incorrectTotal || !shouldRenderProgressBar()) {
    return {
      tick: () => {},
    };
  }

  const logStream = getLogStream('stdout');

  if (logStream.isTTY) {
    return new ProgressBar(`fetching ${name} :bar :percent :etas`, {
      complete: '█',
      incomplete: '░',
      width: 20,
      total,
    });
  }

  let pct = null;
  let current = 0;
  return {
    tick: (length) => {
      current += length;
      const next = Math.round((current / total) * 100);
      if (next > pct) {
        pct = next;
        logStream.write(`fetching ${name} ${pct}%\n`);
      }
    },
  };
}

function npmCache() {
  const env = process.env;
  return (
    env.npm_config_cache ||
    env.npm_config_cache_folder ||
    env.npm_config_yarn_offline_mirror ||
    (env.APPDATA ? path.join(env.APPDATA, 'npm-cache') : path.join(os.homedir(), '.npm'))
  );
}

function getCachedPath(url) {
  const digest = crypto.createHash('md5').update(url).digest('hex').slice(0, 6);

  return path.join(
    npmCache(),
    'sentry-cli',
    `${digest}-${path.basename(url).replace(/[^a-zA-Z0-9.]+/g, '-')}`
  );
}

function getTempFile(cached) {
  return `${cached}.${process.pid}-${Math.random().toString(16).slice(2)}.tmp`;
}

function validateChecksum(tempPath, name, logger) {
  let storedHash;
  try {
    const checksums = fs.readFileSync(path.join(__dirname, '../checksums.txt'), 'utf8');
    const entries = checksums.split('\n');
    for (let i = 0; i < entries.length; i++) {
      const [key, value] = entries[i].split('=');
      if (key === name) {
        storedHash = value;
        break;
      }
    }
  } catch (e) {
    logger.log(
      'Checksums are generated when the package is published to npm. They are not available directly in the source repository. Skipping validation.'
    );
    return;
  }

  if (!storedHash) {
    logger.log(`Checksum for ${name} not found, skipping validation.`);
    return;
  }

  const currentHash = crypto.createHash('sha256').update(fs.readFileSync(tempPath)).digest('hex');

  if (storedHash !== currentHash) {
    fs.unlinkSync(tempPath);
    throw new Error(
      `Checksum validation for ${name} failed.\nExpected: ${storedHash}\nReceived: ${currentHash}`
    );
  } else {
    logger.log('Checksum validation passed.');
  }
}

function checkVersion() {
  return helper.execute(['--version']).then((output) => {
    const version = output.replace('sentry-cli ', '').trim();
    const expected = process.env.SENTRYCLI_LOCAL_CDNURL ? 'DEV' : pkgInfo.version;
    if (version !== expected) {
      throw new Error(`Unexpected sentry-cli version "${version}", expected "${expected}"`);
    }
  });
}

function downloadBinary(logger = ttyLogger) {
  if (process.env.SENTRYCLI_SKIP_DOWNLOAD === '1') {
    logger.log(`Skipping download because SENTRYCLI_SKIP_DOWNLOAD=1 detected.`);
    return Promise.resolve();
  }

  const arch = os.arch();
  const platform = os.platform();
  const outputPath = helper.getPath();

  if (process.env.SENTRYCLI_USE_LOCAL === '1') {
    try {
      const binPath = which.sync('sentry-cli');
      logger.log(`Using local binary: ${binPath}`);
      fs.copyFileSync(binPath, outputPath);
      return Promise.resolve();
    } catch (e) {
      throw new Error(
        'Configured installation of local binary, but it was not found.' +
          'Make sure that `sentry-cli` executable is available in your $PATH or disable SENTRYCLI_USE_LOCAL env variable.'
      );
    }
  }

  const downloadUrl = getDownloadUrl(platform, arch);
  if (!downloadUrl) {
    return Promise.reject(new Error(`Unsupported target ${platform}-${arch}`));
  }

  const cachedPath = getCachedPath(downloadUrl);
  if (fs.existsSync(cachedPath)) {
    logger.log(`Using cached binary: ${cachedPath}`);
    fs.copyFileSync(cachedPath, outputPath);
    return Promise.resolve();
  }

  const proxyUrl = Proxy.getProxyForUrl(downloadUrl);
  const agent = proxyUrl ? new HttpsProxyAgent(proxyUrl) : null;

  logger.log(`Downloading from ${downloadUrl}`);

  if (proxyUrl) {
    logger.log(`Using proxy URL: ${proxyUrl}`);
  }

  return fetch(downloadUrl, {
    agent,
    compress: false,
    headers: {
      'accept-encoding': 'gzip, deflate, br',
    },
    redirect: 'follow',
  })
    .then((response) => {
      if (!response.ok) {
        throw new Error(
          `Unable to download sentry-cli binary from ${downloadUrl}.\nServer returned ${response.status}: ${response.statusText}.`
        );
      }

      const contentEncoding = response.headers.get('content-encoding');
      let decompressor;
      if (/\bgzip\b/.test(contentEncoding)) {
        decompressor = zlib.createGunzip();
      } else if (/\bdeflate\b/.test(contentEncoding)) {
        decompressor = zlib.createInflate();
      } else if (/\bbr\b/.test(contentEncoding)) {
        decompressor = zlib.createBrotliDecompress();
      } else {
        decompressor = new stream.PassThrough();
      }
      const name = downloadUrl.match(/.*\/(.*?)$/)[1];
      const total = parseInt(response.headers.get('content-length'), 10);
      const progressBar = createProgressBar(name, total);
      const tempPath = getTempFile(cachedPath);
      mkdirp.sync(path.dirname(tempPath));

      return new Promise((resolve, reject) => {
        response.body
          .on('error', (e) => reject(e))
          .on('data', (chunk) => progressBar.tick(chunk.length))
          .pipe(decompressor)
          .pipe(fs.createWriteStream(tempPath, { mode: '0755' }))
          .on('error', (e) => reject(e))
          .on('close', () => resolve());
      }).then(() => {
        if (process.env.SENTRYCLI_SKIP_CHECKSUM_VALIDATION !== '1') {
          validateChecksum(tempPath, name, logger);
        }
        fs.copyFileSync(tempPath, cachedPath);
        fs.copyFileSync(tempPath, outputPath);
        fs.unlinkSync(tempPath);
      });
    })
    .then(() => {
      return checkVersion();
    })
    .catch((error) => {
      if (error instanceof fetch.FetchError) {
        throw new Error(
          `Unable to download sentry-cli binary from ${downloadUrl}.\nError code: ${error.code}`
        );
      } else {
        throw error;
      }
    });
}

module.exports = {
  downloadBinary,
};
