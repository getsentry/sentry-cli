/* eslint-env jest */
/* eslint-disable global-require */

const SentryCli = require('../cli');

describe('SentryCli', () => {
  test('call sentry-cli --version', () => {
    expect.assertions(1);
    const cli = new SentryCli();
    return cli.execute(['--version']).then(() => {
      expect(true).toBe(true);
    });
  });

  test('call sentry-cli with wrong command', () => {
    expect.assertions(1);
    const cli = new SentryCli();
    return cli.execute(['--1version']).catch(e => {
      expect(e.message).toMatch('Command failed:');
    });
  });
});
