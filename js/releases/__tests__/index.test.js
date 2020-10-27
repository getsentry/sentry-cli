/* eslint-env jest */

const SentryCli = require('../..');

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
    let cli;
    let mockExecute;
    beforeAll(() => {
      mockExecute = jest.fn(async () => {});
      jest.doMock('../../helper', () => ({
        ...jest.requireActual('../../helper'),
        execute: mockExecute,
      }));
    });
    beforeEach(() => {
      mockExecute.mockClear();
      // eslint-disable-next-line global-require
      const SentryCliLocal = require('../..');
      cli = new SentryCliLocal();
    });
    describe('new', () => {
      test('without projects', async () => {
        await cli.releases.new('my-version');
        expect(mockExecute).toHaveBeenCalledWith(
          ['releases', 'new', 'my-version'],
          null,
          false,
          undefined,
          { silent: false }
        );
      });
      test('with projects', async () => {
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
    describe('uploadSourceMaps', () => {
      test('without projects', async () => {
        await cli.releases.uploadSourceMaps('my-version', { include: ['path'] });
        expect(mockExecute).toHaveBeenCalledWith(
          [
            'releases',
            'files',
            'my-version',
            'upload-sourcemaps',
            'path',
            '--ignore',
            'node_modules',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });
      test('with projects', async () => {
        await cli.releases.uploadSourceMaps('my-version', {
          include: ['path'],
          projects: ['proj-a', 'proj-b'],
        });
        expect(mockExecute).toHaveBeenCalledWith(
          [
            'releases',
            '-p',
            'proj-a',
            '-p',
            'proj-b',
            'files',
            'my-version',
            'upload-sourcemaps',
            'path',
            '--ignore',
            'node_modules',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });
    });
  });
});
