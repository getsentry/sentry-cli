'use strict';

/* global Promise */

var childProcess = require('child_process');
var os = require('os');
var path = require('path');
var pkgInfo = require('../package.json');

var DEFAULT_IGNORE = ['node_modules'];
var SOURCEMAPS_OPTIONS = {
  ignore: {
    param: '--ignore',
    type: 'array'
  },
  ignoreFile: {
    param: '--ignore-file',
    type: 'string'
  },
  noSourceMapReference: {
    param: '--no-sourcemap-reference',
    type: 'boolean'
  },
  stripPrefix: {
    param: '--strip-prefix',
    type: 'array'
  },
  stripCommonPrefix: {
    param: '--strip-common-prefix',
    type: 'array'
  },
  validate: {
    param: '--validate',
    type: 'boolean'
  },
  urlPrefix: {
    param: '--url-prefix',
    type: 'string'
  },
  ext: {
    param: '--ext',
    type: 'string'
  }
};

var binaryPath =
  os.platform() === 'win32'
    ? path.resolve(__dirname, '..\\bin\\sentry-cli.exe')
    : path.resolve(__dirname, '../sentry-cli');

function transformOption(option, values) {
  if (Array.isArray(values)) {
    return values
      .map(function(value) {
        return [option.param, value];
      })
      .reduce(function(acc, value) {
        return acc.concat(value);
      }, []);
  }
  return [option.param, values];
}

function normalizeOptions(options) {
  return Object.keys(SOURCEMAPS_OPTIONS).reduce(function(newOptions, sourceMapOption) {
    if (options[sourceMapOption] === undefined) return newOptions;

    if (SOURCEMAPS_OPTIONS[sourceMapOption].type === 'array') {
      if (!Array.isArray(options[sourceMapOption])) {
        throw new Error(sourceMapOption + ' should be an array');
      }
      return newOptions.concat(
        transformOption(SOURCEMAPS_OPTIONS[sourceMapOption], options[sourceMapOption])
      );
    } else if (SOURCEMAPS_OPTIONS[sourceMapOption].type === 'boolean') {
      if (typeof options[sourceMapOption] !== 'boolean') {
        throw new Error(sourceMapOption + ' should be a bool');
      }
      if (options[sourceMapOption]) {
        // if it's true
        return newOptions.concat([SOURCEMAPS_OPTIONS[sourceMapOption].param]);
      }
      return newOptions;
    }
    return newOptions.concat(
      SOURCEMAPS_OPTIONS[sourceMapOption].param,
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

      return this.execute(this.prepareCommand(command, options));
    }, this)
  );
};

SentryCli.prototype.prepareCommand = function(command, options) {
  var newOptions = options || {};
  var newCommand = command.concat(normalizeOptions(newOptions));

  if (!newOptions.ignoreFile && !newOptions.ignore) {
    newCommand = newCommand.concat(
      transformOption(SOURCEMAPS_OPTIONS.ignore, DEFAULT_IGNORE)
    );
  }

  return newCommand;
};

SentryCli.getVersion = function() {
  return pkgInfo.version;
};

SentryCli.getPath = function() {
  return binaryPath;
};

module.exports = SentryCli;
