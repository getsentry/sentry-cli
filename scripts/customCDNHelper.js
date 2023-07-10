/**
 * Helper script for setting up custom CDN when installing `sentry-cli` via its npm wrapper `@sentry/cli`
 */

const childProcess = require('child_process');
const fs = require('fs');
const os = require('os');
const path = require('path');

const rawArch = os.arch();
const platform = os.platform();

const archStrings = {
  x64: 'x86_64',
  x86: 'i686',
  ia32: 'i686',
  arm64: 'aarch64',
  arm: 'armv7',
};
const arch = archStrings[rawArch] || rawArch;

const distStrings = {
  darwin: 'Darwin-universal',
  win32: `Windows-${arch}.exe`,
  linux: `Linux-${arch}`,
  freebsd: `Linux-${arch}`,
};
const dist = distStrings[platform];

if (!dist) {
  throw new Error(
    `Current platform and archtitecture is not supported. Got: ${platform} ${rawArch}`
  );
}

const currentPathToBinary = childProcess
  .execSync('which sentry-cli')
  .toString()
  .trim();

const version = childProcess
  .execSync('sentry-cli --version')
  .toString()
  .trim()
  .replace('sentry-cli ', '');

const newPathToBinary = path.join(process.cwd(), `${version}/sentry-cli-${dist}`);

if (!fs.existsSync(version)) {
  fs.mkdirSync(version);
}
fs.copyFileSync(currentPathToBinary, newPathToBinary);
console.log(`sentry-cli binary successfully moved to ${newPathToBinary}.`);
