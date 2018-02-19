/* eslint-env jest */

const os = require('os');
const SentryCli = require('../');

describe('SentryCli', () => {
  test('call getPath', () => {
    const pattern = os.platform() === 'win32' ? /sentry-cli.exe$/ : /sentry-cli$/;
    expect(SentryCli.getPath()).toMatch(pattern);
  });
});
