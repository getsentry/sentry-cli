function t(t) {
  return '[object Object]' === Object.prototype.toString.call(t);
}
function s() {
  return (
    '[object process]' ===
    Object.prototype.toString.call('undefined' != typeof process ? process : 0)
  );
}
const e = {};
function i() {
  return s()
    ? global
    : 'undefined' != typeof window
    ? window
    : 'undefined' != typeof self
    ? self
    : e;
}
function n() {
  const t = i(),
    s = t.crypto || t.msCrypto;
  if (void 0 !== s && s.getRandomValues) {
    const t = new Uint16Array(8);
    s.getRandomValues(t), (t[3] = (4095 & t[3]) | 16384), (t[4] = (16383 & t[4]) | 32768);
    const e = (t) => {
      let s = t.toString(16);
      for (; s.length < 4; ) s = `0${s}`;
      return s;
    };
    return e(t[0]) + e(t[1]) + e(t[2]) + e(t[3]) + e(t[4]) + e(t[5]) + e(t[6]) + e(t[7]);
  }
  return 'xxxxxxxxxxxx4xxxyxxxxxxxxxxxxxxx'.replace(/[xy]/g, (t) => {
    const s = (16 * Math.random()) | 0;
    return ('x' === t ? s : (3 & s) | 8).toString(16);
  });
}
function r(t) {
  const s = i();
  if (!('console' in s)) return t();
  const e = s.console,
    n = {};
  ['debug', 'info', 'warn', 'error', 'log', 'assert'].forEach((t) => {
    t in s.console &&
      e[t].__sentry_original__ &&
      ((n[t] = e[t]), (e[t] = e[t].__sentry_original__));
  });
  const r = t();
  return (
    Object.keys(n).forEach((t) => {
      e[t] = n[t];
    }),
    r
  );
}
function a(s) {
  if (t(s)) {
    const t = s,
      e = {};
    for (const s of Object.keys(t)) void 0 !== t[s] && (e[s] = a(t[s]));
    return e;
  }
  return Array.isArray(s) ? s.map(a) : s;
}
(new (class {
  constructor(t = 'Global') {
    (this._name = t), (this.enabled = !1), (this._global = i());
  }
  log(...t) {
    this.enabled || r(() => this._global.console.log(`Sentry ${this._name} [Log]: ${t.join(' ')}`));
  }
  warn(...t) {
    this.enabled ||
      r(() => this._global.console.warn(`Sentry ${this._name} [Warn]: ${t.join(' ')}`));
  }
  error(...t) {
    this.enabled ||
      r(() => this._global.console.error(`Sentry ${this._name} [Error]: ${t.join(' ')}`));
  }
})().enabled = !0),
  i();
const o = { nowSeconds: () => Date.now() / 1e3 };
const c = s()
    ? (function () {
        try {
          return ((t = module), (s = 'perf_hooks'), t.require(s)).performance;
        } catch (t) {
          return;
        }
        var t, s;
      })()
    : (function () {
        const { performance: t } = i();
        if (!t || !t.now) return;
        return { now: () => t.now(), timeOrigin: Date.now() - t.now() };
      })(),
  h = void 0 === c ? o : { nowSeconds: () => (c.timeOrigin + c.now()) / 1e3 },
  u = o.nowSeconds.bind(o);
h.nowSeconds.bind(h),
  (() => {
    const { performance: t } = i();
    if (t) t.timeOrigin ? t.timeOrigin : (t.timing && t.timing.navigationStart) || Date.now();
  })();
class d {
  constructor({ maxBreadcrumbs: t, beforeBreadcrumb: s } = {}) {
    (this.breadcrumbs = []),
      (this.user = {}),
      (this.tags = {}),
      (this.extra = {}),
      (this.contexts = {}),
      (this._notifyingListeners = !1),
      (this._scopeListeners = []),
      (this._eventProcessors = []),
      (this._maxBreadcrumbs = null != t ? t : 100),
      (this._beforeBreadcrumb = s || ((t) => t));
  }
  clone() {
    const t = new d();
    return (
      (t.breadcrumbs = [...this.breadcrumbs]),
      (t.tags = Object.assign({}, this.tags)),
      (t.extra = Object.assign({}, this.extra)),
      (t.contexts = Object.assign({}, this.contexts)),
      (t.user = this.user),
      (t.level = this.level),
      (t.span = this.span),
      (t.session = this.session),
      (t.transactionName = this.transactionName),
      (t.fingerprint = this.fingerprint),
      (t._eventProcessors = [...this._eventProcessors]),
      t
    );
  }
  addScopeListener(t) {
    this._scopeListeners.push(t);
  }
  addEventProcessor(t) {
    return this._eventProcessors.push(t), this;
  }
  setUser(t) {
    return (
      (this.user = t || {}),
      this.session && this.session.update({ user: t }),
      this._notifyScopeListeners(),
      this
    );
  }
  getUser() {
    return this.user;
  }
  setTags(t) {
    return (
      (this.tags = Object.assign(Object.assign({}, this.tags), t)),
      this._notifyScopeListeners(),
      this
    );
  }
  setTag(t, s) {
    return (
      (this.tags = Object.assign(Object.assign({}, this.tags), { [t]: s })),
      this._notifyScopeListeners(),
      this
    );
  }
  setExtras(t) {
    return (
      (this.extra = Object.assign(Object.assign({}, this.extra), t)),
      this._notifyScopeListeners(),
      this
    );
  }
  setExtra(t, s) {
    return (
      (this.extra = Object.assign(Object.assign({}, this.extra), { [t]: s })),
      this._notifyScopeListeners(),
      this
    );
  }
  setFingerprint(t) {
    return (this.fingerprint = t), this._notifyScopeListeners(), this;
  }
  setLevel(t) {
    return (this.level = t), this._notifyScopeListeners(), this;
  }
  setTransactionName(t) {
    return (this.transactionName = t), this._notifyScopeListeners(), this;
  }
  setTransaction(t) {
    return this.setTransactionName(t);
  }
  setContext(t, s) {
    return (
      null === s
        ? delete this.contexts[t]
        : (this.contexts = Object.assign(Object.assign({}, this.contexts), { [t]: s })),
      this._notifyScopeListeners(),
      this
    );
  }
  setSpan(t) {
    return (this.span = t), this._notifyScopeListeners(), this;
  }
  getSpan() {
    return this.span;
  }
  getTransaction() {
    var t;
    const s = this.getSpan();
    return (null == s ? void 0 : s.transaction)
      ? null == s
        ? void 0
        : s.transaction
      : (null === (t = null == s ? void 0 : s.spanRecorder) || void 0 === t ? void 0 : t.spans[0])
      ? s.spanRecorder.spans[0]
      : void 0;
  }
  setSession(t) {
    return t ? (this.session = t) : delete this.session, this._notifyScopeListeners(), this;
  }
  getSession() {
    return this.session;
  }
  update(s) {
    if (!s) return this;
    if ('function' == typeof s) {
      const t = s(this);
      return t instanceof d ? t : this;
    }
    return (
      s instanceof d
        ? ((this.tags = Object.assign(Object.assign({}, this.tags), s.tags)),
          (this.extra = Object.assign(Object.assign({}, this.extra), s.extra)),
          (this.contexts = Object.assign(Object.assign({}, this.contexts), s.contexts)),
          s.user && Object.keys(s.user).length && (this.user = s.user),
          s.level && (this.level = s.level),
          s.fingerprint && (this.fingerprint = s.fingerprint))
        : t(s) &&
          ((s = s),
          (this.tags = Object.assign(Object.assign({}, this.tags), s.tags)),
          (this.extra = Object.assign(Object.assign({}, this.extra), s.extra)),
          (this.contexts = Object.assign(Object.assign({}, this.contexts), s.contexts)),
          s.user && (this.user = s.user),
          s.level && (this.level = s.level),
          s.fingerprint && (this.fingerprint = s.fingerprint)),
      this
    );
  }
  clear() {
    return (
      (this.breadcrumbs = []),
      (this.tags = {}),
      (this.extra = {}),
      (this.user = {}),
      (this.contexts = {}),
      (this.level = void 0),
      (this.transactionName = void 0),
      (this.fingerprint = void 0),
      (this.span = void 0),
      (this.session = void 0),
      this._notifyScopeListeners(),
      this
    );
  }
  addBreadcrumb(t, s) {
    let e = Object.assign({ timestamp: u() }, t);
    if (((e = this._beforeBreadcrumb(e, s)), null !== e)) {
      const t = Math.min(this._maxBreadcrumbs, 100);
      (this.breadcrumbs = [...this.breadcrumbs, e].slice(-t)), this._notifyScopeListeners();
    }
    return this;
  }
  clearBreadcrumbs() {
    return (this.breadcrumbs = []), this._notifyScopeListeners(), this;
  }
  applyToEvent(t, s) {
    var e;
    if (
      (this.extra &&
        Object.keys(this.extra).length &&
        (t.extra = Object.assign(Object.assign({}, this.extra), t.extra)),
      this.tags &&
        Object.keys(this.tags).length &&
        (t.tags = Object.assign(Object.assign({}, this.tags), t.tags)),
      this.user &&
        Object.keys(this.user).length &&
        (t.user = Object.assign(Object.assign({}, this.user), t.user)),
      this.contexts &&
        Object.keys(this.contexts).length &&
        (t.contexts = Object.assign(Object.assign({}, this.contexts), t.contexts)),
      this.level && (t.level = this.level),
      this.transactionName && (t.transaction = this.transactionName),
      this.span)
    ) {
      t.contexts = Object.assign({ trace: this.span.getTraceContext() }, t.contexts);
      const s = null === (e = this.span.transaction) || void 0 === e ? void 0 : e.name;
      s && (t.tags = Object.assign({ transaction: s }, t.tags));
    }
    this._applyFingerprint(t),
      (t.breadcrumbs = [...(t.breadcrumbs || []), ...this.breadcrumbs]),
      (t.breadcrumbs = t.breadcrumbs.length > 0 ? t.breadcrumbs : void 0);
    let i = t;
    for (const t of this._eventProcessors)
      if ('function' == typeof t) {
        const e = t(i, s);
        if (null === e) return null;
        i = e;
      }
    return i;
  }
  _notifyScopeListeners() {
    this._notifyingListeners ||
      ((this._notifyingListeners = !0),
      this._scopeListeners.forEach((t) => {
        t(this);
      }),
      (this._notifyingListeners = !1));
  }
  _applyFingerprint(t) {
    (t.fingerprint = t.fingerprint
      ? Array.isArray(t.fingerprint)
        ? t.fingerprint
        : [t.fingerprint]
      : []),
      this.fingerprint && (t.fingerprint = t.fingerprint.concat(this.fingerprint)),
      t.fingerprint && !t.fingerprint.length && delete t.fingerprint;
  }
}
var g, l, p, f, b, m, x;
!(function (t) {
  (t.Error = 'error'), (t.Session = 'session'), (t.Transaction = 'transaction');
})(g || (g = {})),
  (function (t) {
    (t[(t.None = 0)] = 'None'),
      (t[(t.Error = 1)] = 'Error'),
      (t[(t.Debug = 2)] = 'Debug'),
      (t[(t.Verbose = 3)] = 'Verbose');
  })(l || (l = {})),
  (function (t) {
    (t.Ok = 'ok'), (t.Exited = 'exited'), (t.Crashed = 'crashed'), (t.Abnormal = 'abnormal');
  })(p || (p = {})),
  (function (t) {
    (t.Fatal = 'fatal'),
      (t.Error = 'error'),
      (t.Warning = 'warning'),
      (t.Info = 'info'),
      (t.Debug = 'debug');
  })(f || (f = {})),
  (function (t) {
    t.fromString = function (s) {
      switch (s) {
        case 'fatal':
          return t.Fatal;
        case 'warn':
        case 'warning':
          return t.Warning;
        case 'log':
        case 'info':
          return t.Info;
        case 'debug':
          return t.Debug;
        default:
          return t.Error;
      }
    };
  })(f || (f = {})),
  (function (t) {
    (t.Unknown = 'unknown'),
      (t.Skipped = 'skipped'),
      (t.Success = 'success'),
      (t.RateLimit = 'rate_limit'),
      (t.Invalid = 'invalid'),
      (t.Failed = 'failed');
  })(b || (b = {})),
  (function (t) {
    t.fromHttpCode = function (s) {
      return s >= 200 && s < 300
        ? t.Success
        : 429 === s
        ? t.RateLimit
        : s >= 400 && s < 500
        ? t.Invalid
        : s >= 500
        ? t.Failed
        : t.Unknown;
    };
  })(b || (b = {})),
  (function (t) {
    (t.Explicit = 'explicitly_set'),
      (t.Sampler = 'client_sampler'),
      (t.Rate = 'client_rate'),
      (t.Inheritance = 'inheritance');
  })(m || (m = {})),
  (function (t) {
    (t.Unknown = 'unknown'),
      (t.Skipped = 'skipped'),
      (t.Success = 'success'),
      (t.RateLimit = 'rate_limit'),
      (t.Invalid = 'invalid'),
      (t.Failed = 'failed');
  })(x || (x = {}));
class _ {
  constructor(t) {
    (this.errors = 0),
      (this.sid = n()),
      (this.timestamp = Date.now()),
      (this.started = Date.now()),
      (this.duration = 0),
      (this.status = p.Ok),
      (this.init = !0),
      t && this.update(t);
  }
  update(t = {}) {
    t.user &&
      (t.user.ip_address && (this.ipAddress = t.user.ip_address),
      t.did || (this.did = t.user.id || t.user.email || t.user.username)),
      (this.timestamp = t.timestamp || Date.now()),
      t.sid && (this.sid = 32 === t.sid.length ? t.sid : n()),
      void 0 !== t.init && (this.init = t.init),
      t.did && (this.did = `${t.did}`),
      'number' == typeof t.started && (this.started = t.started),
      'number' == typeof t.duration
        ? (this.duration = t.duration)
        : (this.duration = this.timestamp - this.started),
      t.release && (this.release = t.release),
      t.environment && (this.environment = t.environment),
      t.ipAddress && (this.ipAddress = t.ipAddress),
      t.userAgent && (this.userAgent = t.userAgent),
      'number' == typeof t.errors && (this.errors = t.errors),
      t.status && (this.status = t.status);
  }
  close(t) {
    t
      ? this.update({ status: t })
      : this.status === p.Ok
      ? this.update({ status: p.Exited })
      : this.update();
  }
  toJSON() {
    return a({
      sid: `${this.sid}`,
      init: this.init,
      started: new Date(this.started).toISOString(),
      timestamp: new Date(this.timestamp).toISOString(),
      status: this.status,
      errors: this.errors,
      did: 'number' == typeof this.did || 'string' == typeof this.did ? `${this.did}` : void 0,
      duration: this.duration,
      attrs: a({
        release: this.release,
        environment: this.environment,
        ip_address: this.ipAddress,
        user_agent: this.userAgent,
      }),
    });
  }
}
export { d as Scope, _ as Session };
//# sourceMappingURL=bundle.min.js.map
