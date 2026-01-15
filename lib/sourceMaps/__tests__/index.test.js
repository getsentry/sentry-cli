describe('SentryCli source maps', () => {
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

    describe('inject', () => {
      test('with single path', async () => {
        await cli.sourceMaps.inject({ paths: ['./dist'] });
        expect(mockExecute).toHaveBeenCalledWith(
          ['sourcemaps', 'inject', './dist', '--ignore', 'node_modules'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with multiple paths', async () => {
        await cli.sourceMaps.inject({ paths: ['./dist', './build'] });
        expect(mockExecute).toHaveBeenCalledWith(
          ['sourcemaps', 'inject', './dist', './build', '--ignore', 'node_modules'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with custom ignore patterns', async () => {
        await cli.sourceMaps.inject({
          paths: ['./dist'],
          ignore: ['vendor', '*.test.js'],
        });
        expect(mockExecute).toHaveBeenCalledWith(
          ['sourcemaps', 'inject', './dist', '--ignore', 'vendor', '--ignore', '*.test.js'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with ignoreFile', async () => {
        await cli.sourceMaps.inject({
          paths: ['./dist'],
          ignoreFile: '.gitignore',
        });
        expect(mockExecute).toHaveBeenCalledWith(
          ['sourcemaps', 'inject', './dist', '--ignore-file', '.gitignore'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with custom extensions', async () => {
        await cli.sourceMaps.inject({
          paths: ['./dist'],
          ext: ['js', 'mjs', 'cjs'],
        });
        expect(mockExecute).toHaveBeenCalledWith(
          [
            'sourcemaps',
            'inject',
            './dist',
            '--ignore',
            'node_modules',
            '--ext',
            'js',
            '--ext',
            'mjs',
            '--ext',
            'cjs',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with dryRun', async () => {
        await cli.sourceMaps.inject({
          paths: ['./dist'],
          dryRun: true,
        });
        expect(mockExecute).toHaveBeenCalledWith(
          ['sourcemaps', 'inject', './dist', '--ignore', 'node_modules', '--dry-run'],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('with all options', async () => {
        await cli.sourceMaps.inject({
          paths: ['./dist', './build'],
          ignore: ['vendor'],
          ext: ['js', 'mjs'],
          dryRun: true,
        });
        expect(mockExecute).toHaveBeenCalledWith(
          [
            'sourcemaps',
            'inject',
            './dist',
            './build',
            '--ignore',
            'vendor',
            '--ext',
            'js',
            '--ext',
            'mjs',
            '--dry-run',
          ],
          true,
          false,
          undefined,
          { silent: false }
        );
      });

      test('throws error when paths is not provided', async () => {
        await expect(cli.sourceMaps.inject({})).rejects.toThrow(
          '`options.paths` must be a valid array of paths.'
        );
      });

      test('throws error when paths is not an array', async () => {
        await expect(cli.sourceMaps.inject({ paths: './dist' })).rejects.toThrow(
          '`options.paths` must be a valid array of paths.'
        );
      });

      test('throws error when paths is empty', async () => {
        await expect(cli.sourceMaps.inject({ paths: [] })).rejects.toThrow(
          '`options.paths` must contain at least one path.'
        );
      });
    });
  });
});
