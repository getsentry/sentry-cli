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
      '--strip-prefix',
      'node',
      '--strip-prefix',
      'app',
      '--ignore',
      'node_modules'
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
      '--no-sourcemap-reference',
      '--ignore',
      'node_modules'
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
      '--ext',
      'js',
      '--ignore',
      'node_modules'
    ]);

    expect(cli.prepareCommand(command, { urlPrefix: '~/' })).toEqual([
      'releases',
      'files',
      'release',
      'upload-sourcemaps',
      '/dev/null',
      '--url-prefix',
      '~/',
      '--ignore',
      'node_modules'
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

  // test('call sentry-cli with source map options', function() {
  //   expect.assertions(1)
  //
  //   var cli = new SentryCli()
  //
  //   var command = [
  //     'releases',
  //     'files',
  //     22,
  //     'upload-sourcemaps',
  //     'testinclude',
  //
  //     '--url-prefix',
  //     '~/path',
  //     '--ext',
  //     '.js',
  //     '--ignore',
  //     'node_modules'
  //   ]
  //
  //   return cli.execute(command).then(function() {
  //     expect(true).toBe(true)
  //   })
  // })
});
