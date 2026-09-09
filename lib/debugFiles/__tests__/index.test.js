describe('SentryCli debug files', () => {
  afterEach(() => {
    jest.resetModules();
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
      const { SentryCli: SentryCliLocal } = require('../..');
      cli = new SentryCliLocal();
    });

    describe('prepare', () => {
      test('with single path', async () => {
        await cli.debugFiles.prepare({ path: './dist' });
        expect(mockExecute).toHaveBeenCalledWith(
          ['debug-files', 'prepare', './dist', '--ignore', 'node_modules'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with paths array', async () => {
        await cli.debugFiles.prepare({ paths: ['./dist', './pkg'] });
        expect(mockExecute).toHaveBeenCalledWith(
          ['debug-files', 'prepare', './dist', './pkg', '--ignore', 'node_modules'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with upload false adds --no-upload', async () => {
        await cli.debugFiles.prepare({ path: './dist', upload: false });
        expect(mockExecute).toHaveBeenCalledWith(
          ['debug-files', 'prepare', './dist', '--ignore', 'node_modules', '--no-upload'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with includeSources and wait', async () => {
        await cli.debugFiles.prepare({
          path: './dist',
          includeSources: true,
          wait: true,
        });
        expect(mockExecute).toHaveBeenCalledWith(
          [
            'debug-files',
            'prepare',
            './dist',
            '--ignore',
            'node_modules',
            '--include-sources',
            '--wait',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with dryRun and requireDwarf', async () => {
        await cli.debugFiles.prepare({
          path: './app.wasm',
          dryRun: true,
          requireDwarf: true,
        });
        expect(mockExecute).toHaveBeenCalledWith(
          [
            'debug-files',
            'prepare',
            './app.wasm',
            '--ignore',
            'node_modules',
            '--dry-run',
            '--require-dwarf',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('throws when path is missing', async () => {
        await expect(cli.debugFiles.prepare({})).rejects.toThrow(
          '`options.path` or `options.paths` must contain at least one path.'
        );
      });
    });
  });
});
