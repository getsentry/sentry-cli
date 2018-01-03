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
  rewrite: {
    param: '--rewrite',
    type: 'boolean'
  },
  sourceMapReference: {
    param: '--no-sourcemap-reference',
    type: 'inverted-boolean'
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
    var paramValue = options[sourceMapOption];
    if (paramValue === undefined) {
      return newOptions;
    }

    var paramType = SOURCEMAPS_OPTIONS[sourceMapOption].type;
    var paramName = SOURCEMAPS_OPTIONS[sourceMapOption].param;

    if (paramType === 'array') {
      if (!Array.isArray(paramValue)) {
        throw new Error(sourceMapOption + ' should be an array');
      }
      return newOptions.concat(
        transformOption(SOURCEMAPS_OPTIONS[sourceMapOption], paramValue)
      );
    } else if (paramType === 'boolean' || paramType === 'inverted-boolean') {
      if (typeof paramValue !== 'boolean') {
        throw new Error(sourceMapOption + ' should be a bool');
      }
      if (paramType === 'boolean' && paramValue) {
        return newOptions.concat([paramName]);
      } else if (paramType === 'inverted-boolean' && paramValue === false) {
        return newOptions.concat([paramName]);
      }
      return newOptions;
    }
    return newOptions.concat(paramName, paramValue);
  }, []);
}

function SentryCli(configFile) {
  this.env = {};
  if (typeof configFile === 'string') this.env.SENTRY_PROPERTIES = configFile;
}

SentryCli.prototype.execute = function(args) {
  var env = Object.assign({}, process.env, this.env);
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
        sourcemapPath
      ];

      return this.execute(this.prepareCommand(command, options));
    }, this)
  );
};

SentryCli.prototype.prepareCommand = function(command, options) {
  var newOptions = options || {};

  if (!newOptions.ignoreFile && !newOptions.ignore) {
    newOptions.ignore = DEFAULT_IGNORE;
  }

  return command.concat(normalizeOptions(newOptions));
};

SentryCli.getVersion = function() {
  return pkgInfo.version;
};

SentryCli.getPath = function() {
  return binaryPath;
};

module.exports = SentryCli;
