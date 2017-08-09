var os = require('os');
var childProcess = require('child_process');
var pkgInfo = require('./package.json');

var path = null;
if (os.platform() === 'win32') {
  path = __dirname + '\\bin\\sentry-cli.exe';
} else {
  path = __dirname + '/sentry-cli';
}

exports.getConfigStatus = function() {
  return new Promise(function(resolve, reject) {
    childProcess.execFile(
      path,
      ['info', '--config-status-json'],
      function(err, stdout, stderr) {
        if (err) {
          reject(err);
          return;
        }
        resolve(JSON.parse(stdout));
      });
  });
};

exports.getVersion = function() {
  return pkgInfo.version;
};

exports.getPath = function() {
  return path;
};
