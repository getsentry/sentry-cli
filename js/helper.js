'use strict';

const os = require('os');
const path = require('path');
const childProcess = require('child_process');

const binaryPath =
  os.platform() === 'win32'
    ? path.resolve(__dirname, '..\\bin\\sentry-cli.exe')
    : path.resolve(__dirname, '../sentry-cli');

function transformOption(option, values) {
  if (Array.isArray(values)) {
    return values.reduce((acc, value) => acc.concat([option.param, value]), []);
  }

  return [option.param, values];
}

function normalizeOptions(commandOptions, options) {
  return Object.keys(commandOptions).reduce((newOptions, sourceMapOption) => {
    const paramValue = options[sourceMapOption];
    if (paramValue === undefined) {
      return newOptions;
    }

    const paramType = commandOptions[sourceMapOption].type;
    const paramName = commandOptions[sourceMapOption].param;

    if (paramType === 'array') {
      if (!Array.isArray(paramValue)) {
        throw new Error(`${sourceMapOption} should be an array`);
      }

      return newOptions.concat(
        transformOption(commandOptions[sourceMapOption], paramValue)
      );
    }

    if (paramType === 'boolean' || paramType === 'inverted-boolean') {
      if (typeof paramValue !== 'boolean') {
        throw new Error(`${sourceMapOption} should be a bool`);
      }

      if (paramType === 'boolean' && paramValue) {
        return newOptions.concat([paramName]);
      }

      if (paramType === 'inverted-boolean' && paramValue === false) {
        return newOptions.concat([paramName]);
      }

      return newOptions;
    }

    return newOptions.concat(paramName, paramValue);
  }, []);
}

function prepareCommand(command, commandOptions, options) {
  return command.concat(normalizeOptions(commandOptions || {}, options || {}));
}

function getPath() {
  if (process.env.NODE_ENV === 'test') {
    return path.resolve(__dirname, '__mocks__/sentry-cli');
  }

  return binaryPath;
}

function execute(args) {
  const env = Object.assign({}, process.env);
  return new Promise((resolve, reject) => {
    childProcess.execFile(getPath(), args, { env }, (err, stdout) => {
      if (err) {
        reject(err);
      } else {
        resolve(stdout);
      }
    });
  });
}

module.exports = {
  normalizeOptions,
  prepareCommand,
  getPath,
  execute,
};
