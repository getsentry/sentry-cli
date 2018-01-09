/* eslint-env jest */

var SentryCli = require('..');
var helper = require('../helper');

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

describe('SentryCli', function() {
  test('call sentry-cli --version', function() {
    expect.assertions(1);
    return helper.execute(['--version']).then(function() {
      expect(true).toBe(true);
    });
  });

  test('call sentry-cli with wrong command', function() {
    expect.assertions(1);
    return helper.execute(['fail']).catch(function(e) {
      expect(e.message).toMatch('Command failed:');
    });
  });

  test('call prepare command add default ignore', function() {
    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(helper.prepareCommand(command)).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null'
    ]);
  });

  test('call prepare command with array option', function() {
    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { stripPrefix: ['node', 'app'] })
    ).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--strip-prefix',
      'node',
      '--strip-prefix',
      'app'
    ]);

    // Should throw since it is no array
    expect(function() {
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { stripPrefix: 'node' });
    }).toThrow();
  });

  test('call prepare command with boolean option', function() {
    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: false })
    ).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--no-sourcemap-reference'
    ]);

    expect(
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: true })
    ).toEqual(['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null']);

    expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { rewrite: true })).toEqual(
      ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null', '--rewrite']
    );

    expect(function() {
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { sourceMapReference: 'node' });
    }).toThrow();
  });

  test('call prepare command with string option', function() {
    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { ext: 'js' })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ext',
      'js'
    ]);

    expect(
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { urlPrefix: '~/' })
    ).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--url-prefix',
      '~/'
    ]);

    expect(
      helper.prepareCommand(command, SOURCEMAPS_OPTIONS, { ignoreFile: '/js.ignore' })
    ).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore-file',
      '/js.ignore'
    ]);
  });

  test('call sentry-cli releases propose-version', function() {
    expect.assertions(1);
    var cli = new SentryCli();
    return cli.releases.proposeVersion().then(function(version) {
      expect(version).toBeTruthy();
    });
  });
});
