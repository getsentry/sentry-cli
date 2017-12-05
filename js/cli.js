'use strict';

const childProcess = require('child_process');
const os = require('os');
const path = require('path');
const pkgInfo = require('../package.json');

const DEFAULT_IGNORE = ['node_modules'];

let binaryPath = null;
if (os.platform() === 'win32') {
  binaryPath = path.resolve(`${__dirname}\\..\\bin\\sentry-cli.exe`);
} else {
  binaryPath = path.resolve(`${__dirname}/../sentry-cli`);
}

function transformIgnore(ignore) {
  if (Array.isArray(ignore)) {
    return ignore
      .map(value => ['--ignore', value])
      .reduce((acc, value) => acc.concat(value), []);
  }
  return ['--ignore', ignore];
}

function SentryCli(configFile) {
  this.env = {};
  if (typeof configFile === 'string') this.env.SENTRY_PROPERTIES = configFile;
}

SentryCli.prototype.execute = function(args) {
  const env = this.env;
  return new Promise((resolve, reject) => {
    childProcess.execFile(SentryCli.getPath(), args, { env }, (err, stdout) => {
      if (err) return reject(err);
      // eslint-disable-next-line
      console.log(stdout);
      return resolve();
    });
  });
};

SentryCli.prototype.getConfigStatus = function() {
  return this.execute(['info', '--config-status-json']);
};

SentryCli.prototype.createRelease = function(release) {
  return this.execute(['releases', 'new', release]);
};

SentryCli.prototype.finalizeRelease = function(release) {
  return this.execute(['releases', 'finalize', release]);
};

SentryCli.prototype.uploadSourceMaps = function(options) {
  return Promise.all(
    options.include.map(sourcemapPath => {
      let command = [
        'releases',
        'files',
        options.release,
        'upload-sourcemaps',
        sourcemapPath,
        '--rewrite',
      ];

      if (options.ignoreFile) {
        command = command.concat(['--ignore-file', options.ignoreFile]);
      }

      if (options.ignore) {
        command = command.concat(transformIgnore(options.ignore));
      }

      if (!options.ignoreFile && !options.ignore) {
        command = command.concat(transformIgnore(DEFAULT_IGNORE));
      }

      return this.execute(command);
    })
  );
};

SentryCli.getVersion = function() {
  return pkgInfo.version;
};

SentryCli.getPath = function() {
  return binaryPath;
};

module.exports = SentryCli;
