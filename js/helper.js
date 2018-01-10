'use strict';

/* global Promise */

var os = require('os');
var path = require('path');
var childProcess = require('child_process');

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

var binaryPath =
  os.platform() === 'win32'
    ? path.resolve(__dirname, '..\\bin\\sentry-cli.exe')
    : path.resolve(__dirname, '../sentry-cli');

module.exports = {
  normalizeOptions: function(commandOptions, options) {
    return Object.keys(commandOptions).reduce(function(newOptions, sourceMapOption) {
      var paramValue = options[sourceMapOption];
      if (typeof paramValue === 'undefined') {
        return newOptions;
      }

      var paramType = commandOptions[sourceMapOption].type;
      var paramName = commandOptions[sourceMapOption].param;

      if (paramType === 'array') {
        if (!Array.isArray(paramValue)) {
          throw new Error(sourceMapOption + ' should be an array');
        }
        return newOptions.concat(
          transformOption(commandOptions[sourceMapOption], paramValue)
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
  },

  prepareCommand: function(command, commandOptions, options) {
    return command.concat(this.normalizeOptions(commandOptions || {}, options || {}));
  },

  getPath: function() {
    if (process.env.NODE_ENV === 'test') {
      return path.resolve(__dirname, '__mocks__/sentry-cli');
    }
    return binaryPath;
  },

  execute: function(args) {
    var env = Object.assign({}, process.env);
    var that = this;
    return new Promise(function(resolve, reject) {
      childProcess.execFile(that.getPath(), args, { env: env }, function(err, stdout) {
        if (err) return reject(err);
        // eslint-disable-next-line
        console.log(stdout);
        return resolve(stdout);
      });
    });
  }
};
