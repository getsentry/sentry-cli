/* eslint-env jest */

const SentryCli = require('../');

describe('SentryCli', () => {
  test('call getPath', () => {
    expect(SentryCli.getPath()).toContain('sentry-cli');
  });
});
