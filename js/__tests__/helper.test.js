/* eslint-env jest */

var helper = require('../helper');

var SOURCEMAPS_OPTIONS = require('../releases/options/uploadSourcemaps');

describe('SentryCli helper', function() {
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
});
