'use strict';

/* global Promise */

var childProcess = require('child_process');
var os = require('os');
var path = require('path');
var pkgInfo = require('../package.json');

var DEFAULT_IGNORE = ['node_modules'];
var SOURCEMAPS_OPTIONS = {
  ignore: '--ignore',
  ignoreFile: '--ignore-file',
  noSourceMapReference: '--no-sourcemap-reference',
  stripPrefix: '--strip-prefix',
  stripCommonPrefix: '--strip-common-prefix',
  validate: '--validate',
  urlPrefix: '--url-prefix',
  ext: '--ext'
};

var binaryPath =
  os.platform() === 'win32'
    ? path.resolve(__dirname, '..\\bin\\sentry-cli.exe')
    : path.resolve(__dirname, '../sentry-cli');

function transformOption(option, values) {
  if (Array.isArray(values)) {
    return values
      .map(function(value) {
        return [option, value];
      })
      .reduce(function(acc, value) {
        return acc.concat(value);
      }, []);
  }
  return [option, values];
}

function normalizeOptions(options) {
  var transformableOptions = ['ignore', 'stripPrefix', 'stripCommonPrefix'];

  return Object.keys(SOURCEMAPS_OPTIONS).reduce(function(newOptions, sourceMapOption) {
    if (options[sourceMapOption] === undefined) return newOptions;

    if (transformableOptions.indexOf(sourceMapOption) !== -1) {
      return newOptions.concat(
        transformOption(SOURCEMAPS_OPTIONS[sourceMapOption], options[sourceMapOption])
      );
    } else if (sourceMapOption === 'validate') {
      return newOptions.concat([SOURCEMAPS_OPTIONS[sourceMapOption]]);
    }
    return newOptions.concat(
      SOURCEMAPS_OPTIONS[sourceMapOption],
      options[sourceMapOption]
    );
  }, []);
}

function SentryCli(configFile) {
  this.env = {};
  if (typeof configFile === 'string') this.env.SENTRY_PROPERTIES = configFile;
}

SentryCli.prototype.execute = function(args) {
  var env = this.env;
  return new Promise(function(resolve, reject) {
    childProcess.execFile(SentryCli.getPath(), args, { env: env }, function(err, stdout) {
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
    options.include.map(function(sourcemapPath) {
      var command = [
        'releases',
        'files',
        options.release,
        'upload-sourcemaps',
        sourcemapPath,
        '--rewrite'
      ];

      command = command.concat(normalizeOptions(options));

      if (!options.ignoreFile && !options.ignore) {
        command = command.concat(transformOption('--ignore', DEFAULT_IGNORE));
      }

      return this.execute(command);
    }, this)
  );
};

SentryCli.getVersion = function() {
  return pkgInfo.version;
};

SentryCli.getPath = function() {
  return binaryPath;
};

module.exports = SentryCli;
