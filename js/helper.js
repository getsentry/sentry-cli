'use strict';

const os = require('os');
const path = require('path');
const childProcess = require('child_process');

/**
 * Absolute path to the sentry-cli binary (platform dependant).
 * @type {string}
 */
const binaryPath =
  os.platform() === 'win32'
    ? path.resolve(__dirname, '..\\bin\\sentry-cli.exe')
    : path.resolve(__dirname, '../sentry-cli');

/**
 * Converts the given option into a command line args array.
 *
 * The value can either be an array of values or a single value. The value(s) will be
 * converted to string. If an array is given, the option name is repeated for each value.
 *
 * @example
 * expect(transformOption('--foo', 'a'))
 *   .toEqual(['--foo', 'a'])
 *
 * @example
 * expect(transformOption('--foo', ['a', 'b']))
 *   .toEqual(['--foo', 'a', '--foo', 'b']);
 *
 * @param {string} option The literal name of the option, including dashes.
 * @param {any[]|any} values One or more values for this option.
 * @returns {string[]} An arguments array that can be passed via command line.
 */
function transformOption(option, values) {
  if (Array.isArray(values)) {
    return values.reduce((acc, value) => acc.concat([option.param, String(value)]), []);
  }

  return [option.param, String(values)];
}

/**
 * The javascript type of a command line option.
 * @typedef {'array'|'string'|'boolean'|'inverted-boolean'} OptionType
 */

/**
 * Schema definition of a command line option.
 * @typedef {object} OptionSchema
 * @prop {string} param The flag of the command line option including dashes.
 * @prop {OptionType} type The value type of the command line option.
 */

/**
 * Schema definition for a command.
 * @typedef {Object.<string, OptionSchema>} OptionsSchema
 */

/**
 * Serializes command line options into an arguments array.
 *
 * @param {OptionsSchema} schema An options schema required by the command.
 * @param {object} options An options object according to the schema.
 * @returns {string[]} An arguments array that can be passed via command line.
 */
function serializeOptions(schema, options) {
  return Object.keys(schema).reduce((newOptions, option) => {
    const paramValue = options[option];
    if (paramValue === undefined) {
      return newOptions;
    }

    const paramType = schema[option].type;
    const paramName = schema[option].param;

    if (paramType === 'array') {
      if (!Array.isArray(paramValue)) {
        throw new Error(`${option} should be an array`);
      }

      return newOptions.concat(transformOption(schema[option], paramValue));
    }

    if (paramType === 'boolean' || paramType === 'inverted-boolean') {
      if (typeof paramValue !== 'boolean') {
        throw new Error(`${option} should be a bool`);
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

/**
 * Serializes the command and its options into an arguments array.
 *
 * @param {string} command The literal name of the command.
 * @param {OptionsSchema} [schema] An options schema required by the command.
 * @param {object} [options] An options object according to the schema.
 * @returns {string[]} An arguments array that can be passed via command line.
 */
function prepareCommand(command, schema, options) {
  return command.concat(serializeOptions(schema || {}, options || {}));
}

/**
 * Returns the absolute path to the `sentry-cli` binary.
 * @returns {string}
 */
function getPath() {
  if (process.env.NODE_ENV === 'test') {
    return path.resolve(__dirname, '__mocks__/sentry-cli');
  }

  return binaryPath;
}

/**
 * Runs `sentry-cli` with the given command line arguments.
 *
 * Use {@link prepareCommand} to specify the command and add arguments for command-
 * specific options. For top-level options, use {@link serializeOptions} directly.
 *
 * The returned promise resolves with the standard output of the command invocation
 * including all newlines. In order to parse this output, be sure to trim the output
 * first.
 *
 * If the command failed to execute, the Promise rejects with the error returned by the
 * CLI. This error includes a `code` property with the process exit status.
 *
 * @example
 * const output = await execute(['--version']);
 * expect(output.trim()).toBe('sentry-cli x.y.z');
 *
 * @param {string[]} args Command line arguments passed to `sentry-cli`.
 * @returns {Promise.<string>} A promise that resolves to the standard output.
 */
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
  serializeOptions,
  prepareCommand,
  getPath,
  execute,
};
