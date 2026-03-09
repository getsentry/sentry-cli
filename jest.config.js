module.exports = {
  collectCoverage: true,
  testEnvironment: 'node',
  setupFiles: ['<rootDir>/setupTests.js'],
  testPathIgnorePatterns: ['<rootDir>/src/', '<rootDir>/tests/integration/'],
  transform: {
    '^.+\\.ts$': 'ts-jest',
  },
};
