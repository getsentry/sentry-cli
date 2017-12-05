const os = require('os');
const pkgInfo = require('./package.json');
const SentryCli = require('./js/cli');

exports.default = SentryCli;

let path = null;
if (os.platform() === 'win32') {
  path = `${__dirname}\\bin\\sentry-cli.exe`;
} else {
  path = `${__dirname}/sentry-cli`;
}

exports.getVersion = function() {
  return pkgInfo.version;
};

exports.getPath = function() {
  return path;
};
