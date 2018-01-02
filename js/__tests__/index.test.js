/* eslint-env jest */

var SentryCli = require('..');

describe('SentryCli', function() {
  test('call sentry-cli --version', function() {
    expect.assertions(1);
    var cli = new SentryCli();
    return cli.execute(['--version']).then(function() {
      expect(true).toBe(true);
    });
  });

  test('call sentry-cli with wrong command', function() {
    expect.assertions(1);
    var cli = new SentryCli();
    return cli.execute(['fail']).catch(function(e) {
      expect(e.message).toMatch('Command failed:');
    });
  });

  test('call prepare command add default ignore', function() {
    var cli = new SentryCli();

    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(cli.prepareCommand(command)).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules'
    ]);
  });

  test('call prepare command with array option', function() {
    var cli = new SentryCli();

    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(cli.prepareCommand(command, { stripPrefix: ['node', 'app'] })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules',
      '--strip-prefix',
      'node',
      '--strip-prefix',
      'app'
    ]);

    // Should throw since it is no array
    expect(function() {
      cli.prepareCommand(command, { stripPrefix: 'node' });
    }).toThrow();
  });

  test('call prepare command with boolean option', function() {
    var cli = new SentryCli();

    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(cli.prepareCommand(command, { sourceMapReference: false })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules',
      '--no-sourcemap-reference'
    ]);

    expect(cli.prepareCommand(command, { sourceMapReference: true })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules'
    ]);

    expect(cli.prepareCommand(command, { rewrite: true })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules',
      '--rewrite'
    ]);

    expect(function() {
      cli.prepareCommand(command, { sourceMapReference: 'node' });
    }).toThrow();
  });

  test('call prepare command with string option', function() {
    var cli = new SentryCli();

    var command = ['releases', 'files', 'release', 'upload-sourcemaps', '/dev/null'];

    expect(cli.prepareCommand(command, { ext: 'js' })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules',
      '--ext',
      'js'
    ]);

    expect(cli.prepareCommand(command, { urlPrefix: '~/' })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--ignore',
      'node_modules',
      '--url-prefix',
      '~/'
    ]);

    expect(cli.prepareCommand(command, { ignoreFile: '/js.ignore' })).toEqual([
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
