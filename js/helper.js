'use strict';

const path = require('path');
const childProcess = require('child_process');

/**
 * This convoluted function resolves the path to the `sentry-cli` binary in a
 * way that can't be analysed by @vercel/nft.
 *
 * Without this, the binary can be detected as an asset and included by bundlers
 * that use @vercel/nft.
 * @returns {string} The path to the sentry-cli binary
 */
function getBinaryPath() {
  const parts = [];
  parts.push(__dirname);
  parts.push('..');
  parts.push(`sentry-cli${process.platform === 'win32' ? '.exe' : ''}`);
  return path.resolve(...parts);
}

/**
 * Absolute path to the sentry-cli binary (platform dependent).
 * @type {string}
 */
let binaryPath = getBinaryPath();

/**
 * Overrides the default binary path with a mock value, useful for testing.
 *
 * @param {string} mockPath The new path to the mock sentry-cli binary
 */
function mockBinaryPath(mockPath) {
  binaryPath = mockPath;
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

      return newOptions.concat(
        paramValue.reduce((acc, value) => acc.concat([paramName, String(value)]), [])
      );
    }

    if (paramType === 'boolean') {
      if (typeof paramValue !== 'boolean') {
        throw new Error(`${option} should be a bool`);
      }

      const invertedParamName = schema[option].invertedParam;

      if (paramValue && paramName !== undefined) {
        return newOptions.concat([paramName]);
      }

      if (!paramValue && invertedParamName !== undefined) {
        return newOptions.concat([invertedParamName]);
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
 * @param {boolean} live We inherit stdio to display `sentry-cli` output directly.
 * @param {boolean} silent Disable stdout for silents build (CI/Webpack Stats, ...)
 * @param {string} [configFile] Relative or absolute path to the configuration file.
 * @param {Object} [config] More configuration to pass to the CLI
 * @returns {Promise.<string>} A promise that resolves to the standard output.
 */
function execute(args, live, silent, configFile, config = {}) {
  const env = { ...process.env };
  if (configFile) {
    env.SENTRY_PROPERTIES = configFile;
  }
  if (config.url) {
    env.SENTRY_URL = config.url;
  }
  if (config.authToken) {
    env.SENTRY_AUTH_TOKEN = config.authToken;
  }
  if (config.apiKey) {
    env.SENTRY_API_KEY = config.apiKey;
  }
  if (config.dsn) {
    env.SENTRY_DSN = config.dsn;
  }
  if (config.org) {
    env.SENTRY_ORG = config.org;
  }
  if (config.project) {
    env.SENTRY_PROJECT = config.project;
  }
  if (config.vcsRemote) {
    env.SENTRY_VCS_REMOTE = config.vcsRemote;
  }
  if (config.customHeader) {
    env.CUSTOM_HEADER = config.customHeader;
  }
  return new Promise((resolve, reject) => {
    if (live === true) {
      const output = silent ? 'ignore' : 'inherit';
      const pid = childProcess.spawn(getPath(), args, {
        env,
        // stdin, stdout, stderr
        stdio: ['ignore', output, output],
      });
      pid.on('exit', () => {
        resolve();
      });
    } else {
      childProcess.execFile(getPath(), args, { env }, (err, stdout) => {
        if (err) {
          reject(err);
        } else {
          resolve(stdout);
        }
      });
    }
  });
}

function getProjectFlagsFromOptions({ projects = [] } = {}) {
  return projects.reduce((flags, project) => flags.concat('-p', project), []);
}

module.exports = {
  execute,
  getPath,
  getProjectFlagsFromOptions,
  mockBinaryPath,
  prepareCommand,
  serializeOptions,
};
