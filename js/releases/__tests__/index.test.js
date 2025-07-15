const SentryCli = require('../..');

describe('SentryCli releases', () => {
  afterEach(() => {
    jest.resetModules();
  });
  test('call sentry-cli releases propose-version', () => {
    expect.assertions(1);
    const cli = new SentryCli();
    return cli.releases.proposeVersion().then((version) => expect(version).toBeTruthy());
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

      test('handles multiple include entries', async () => {
        expect.assertions(3);

        const paths = ['path', 'other-path'];
        await cli.releases.uploadSourceMaps('my-version', { include: paths });

        expect(mockExecute).toHaveBeenCalledTimes(2);
        paths.forEach((path) =>
          expect(mockExecute).toHaveBeenCalledWith(
            [
              'releases',
              'files',
              'my-version',
              'upload-sourcemaps',
              path,
              '--ignore',
              'node_modules',
            ],
            true,
            false,
            undefined,
            { silent: false }
          )
        );
      });

      test('handles object-type include entries', async () => {
        expect.assertions(3);

        const paths = [{ paths: ['some-path'], ignore: ['not-me'] }, 'other-path'];
        await cli.releases.uploadSourceMaps('my-version', { include: paths });

        expect(mockExecute).toHaveBeenCalledTimes(2);

        expect(mockExecute).toHaveBeenCalledWith(
          [
            'releases',
            'files',
            'my-version',
            'upload-sourcemaps',
            'some-path',
            '--ignore',
            'not-me', // note how this has been overridden
          ],
          true,
          false,
          undefined,
          { silent: false }
        );

        expect(mockExecute).toHaveBeenCalledWith(
          [
            'releases',
            'files',
            'my-version',
            'upload-sourcemaps',
            'other-path',
            '--ignore',
            'node_modules',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test.each([true, false, 'rejectOnError'])('handles live mode %s', async (live) => {
        await cli.releases.uploadSourceMaps('my-version', { include: ['path'], live });
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
          live,
          false,
          undefined,
          { silent: false }
        );
      });
    });
  });
});
