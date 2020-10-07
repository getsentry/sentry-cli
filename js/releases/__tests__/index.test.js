/* eslint-env jest */

const SentryCli = require('../../');

describe('SentryCli releases', () => {
  afterEach(() => {
    jest.resetModules();
  });
  test('call sentry-cli releases propose-version', () => {
    expect.assertions(1);
    const cli = new SentryCli();
    return cli.releases.proposeVersion().then(version => expect(version).toBeTruthy());
  });

  describe('with mock', () => {
    let cli, mockExecute;
    beforeEach(() => {
      mockExecute = jest.fn(async () => {});
      jest.doMock('../../helper', () => ({
        execute: mockExecute,
      }));
      const SentryCli = require('../../');
      cli = new SentryCli();
    });
    test('new without projects', async () => {
      await cli.releases.new('my-version');
      expect(mockExecute).toHaveBeenCalledWith(
        ['releases', 'new', 'my-version'],
        null,
        false,
        undefined,
        { silent: false }
      );
    });
    test('new with projects', async () => {
      await cli.releases.new('my-version', { projects: ['proj-a', 'proj-b'] });
      expect(mockExecute).toHaveBeenCalledWith(
        ['releases', 'new', 'my-version', '-p', 'proj-a', '-p', 'proj-b'],
        null,
        false,
        undefined,
        { silent: false }
      );
    });
  });
});
