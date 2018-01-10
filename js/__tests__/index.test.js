/* eslint-env jest */

var SentryCli = require('../');

describe('SentryCli', function() {
  test('call getPath', function() {
    expect(SentryCli.getPath()).toContain('sentry-cli');
  });
});
