module.exports = {
  collectCoverage: true,
  testEnvironment: 'node',
  setupFiles: ['./setupTests.js'],
  testPathIgnorePatterns: ['./src/', './tests/integration/'],
  transform: {
    '^.+\\.ts$': 'ts-jest',
  },
};
