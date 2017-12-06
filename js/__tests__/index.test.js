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
});
