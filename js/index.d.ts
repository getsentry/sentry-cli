/*
  Typings for @sentry/cli
*/
declare module '@sentry/cli' {
  export interface SentryCliOptions {
    url?: string;
    authToken?: string;
    apiKey?: string;
    dsn?: string;
    org?: string;
    project?: string;
    vscRemote?: string;
    silent?: boolean;
  }

  export interface SentryCliUploadSourceMapsOptions {
    include: string | string[];
    ignore?: string[];
    ignoreFile?: string | null;
    rewrite?: boolean;
    sourceMapReference?: boolean;
    stripPrefix?: string[];
    stripCommonPrefix?: boolean;
    validate?: boolean;
    urlPrefix?: string;
    urlSuffix?: string;
    ext?: string[];
  }

  export interface SentryCliNewDeployOptions {
    env: string;
    started?: number;
    finished?: number;
    time?: number;
    name?: string;
    url?: string;
  }

  export interface SentryCliCommitsOptions {
    auto?: boolean;
    repo?: string;
    commit?: string;
    previousCommit?: string;
    ignoreMissing?: boolean;
  }

  export interface SentryCliReleases {
    ['new'](
      release: string,
      options?: { projects: string[] } | string[]
    ): Promise<string>;

    setCommits(
      release: string,
      options: SentryCliCommitsOptions
    ): Promise<string>;

    finalize(release: string): Promise<string>

    proposeVersion(): Promise<string>

    uploadSourceMaps(
      release: string,
      options: SentryCliUploadSourceMapsOptions
    ): Promise<string>

    listDeploys(release: string): Promise<string>;

    newDeploy(
      release: string,
      options: SentryCliNewDeployOptions
    ): Promise<string>

    execute(args: string[], live: boolean): Promise<string>;
  }

  export default class SentryCli {
    constructor(configFile?: string | null, options?: SentryCliOptions)

    public configFile?: string;
    public options?: SentryCliOptions;
    public releases: SentryCliReleases

    public static getVersion(): string
    public static getPath(): string
    public execute(args: string[], live: boolean): Promise<string>
  }
}
