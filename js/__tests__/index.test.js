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
  //     '--rewrite',
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
