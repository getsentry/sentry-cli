"use strict";
exports.id = 882;
exports.ids = [882];
exports.modules = {

/***/ 4882:
/***/ ((__unused_webpack_module, __unused_webpack___webpack_exports__, __webpack_require__) => {

/* harmony import */ var _sentry_nextjs__WEBPACK_IMPORTED_MODULE_0__ = __webpack_require__(8097);
/* harmony import */ var _sentry_nextjs__WEBPACK_IMPORTED_MODULE_0___default = /*#__PURE__*/__webpack_require__.n(_sentry_nextjs__WEBPACK_IMPORTED_MODULE_0__);
var _sentryCollisionFreeGlobalObject = "undefined" === "undefined" ? global : window;
_sentryCollisionFreeGlobalObject["__sentryRewritesTunnelPath__"] = "/test";
_sentryCollisionFreeGlobalObject["SENTRY_RELEASE"] = {
    "id": "H7WYeNZ2HObPMd59_6n2m"
};
_sentryCollisionFreeGlobalObject["__rewriteFramesDistDir__"] = ".next";

_sentry_nextjs__WEBPACK_IMPORTED_MODULE_0__.init({
    dsn: "https://5ca6c435afc347aaa9a5e6fe9113c11f@o1163812.ingest.sentry.io/6762530",
    // We recommend adjusting this value in production, or using tracesSampler
    // for finer control
    tracesSampleRate: 1,
    // ...
    // Note: if you want to override the automatic release value, do not set a
    // `release` value here - use the environment variable `SENTRY_RELEASE`, so
    // that it will also get attached to your source maps
    debug: true,
    // release: "23.01.2023.6",
    environment: process.env.VERCEL ? "vercel" : "local",
    beforeSend: (event)=>{
        console.log(event);
        return event;
    }
});


/***/ })

};
;
//# sourceMappingURL=882.js.map