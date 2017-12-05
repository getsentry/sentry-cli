const childProcess = require('child_process');
const cli = require('../index.js');

const DEFAULT_IGNORE = ['node_modules'];

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
    childProcess.execFile(cli.getPath(), args, { env }, (err, stdout) => {
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
    options.include.map(path => {
      let command = [
        'releases',
        'files',
        options.release,
        'upload-sourcemaps',
        path,
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

module.exports = SentryCli;
