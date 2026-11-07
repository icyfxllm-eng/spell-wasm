// SpellStore: the web layer's one interface to the iOS StoreKit 2 purchase
// adapter (CC-LAUNCH-EN F4). It loads on every platform and exposes
// window.SpellStore. On iOS with the SpellStorePlugin present it routes to
// StoreKit. Everywhere else (web, PWA) it is the stub: it owns nothing, has no
// products, and every purchase or restore rejects UNAVAILABLE, so the page
// shows "Get the app".
//
// It is an input adapter only. Which product ids are owned goes to the Rust
// core (src/entitlements_adapter.rs), and that core decides what they unlock.
//
// @typedef {{ id:string, displayName:string, displayPrice:string }} StoreProduct
// @typedef {{ status:string, ids:string[] }} PurchaseResult  status: success|cancelled|pending|unverified|unknown
(function () {
  'use strict';

  var _proxy;
  function plugin() {
    var cap = window.Capacitor;
    if (!cap) return undefined;
    if (!_proxy && typeof cap.registerPlugin === 'function') {
      _proxy = cap.registerPlugin('SpellStore');
    }
    return _proxy || (cap.Plugins && cap.Plugins.SpellStore);
  }

  function available() {
    var cap = window.Capacitor;
    return !!(cap && cap.isNativePlatform && cap.isNativePlatform() && plugin());
  }

  function ids(r) {
    return r && Array.isArray(r.ids) ? r.ids.filter(function (x) { return typeof x === 'string'; }) : [];
  }

  window.SpellStore = {
    /** @returns {boolean} */
    available: available,

    /**
     * The verified, unrevoked product ids owned right now. Never rejects:
     * resolves [] off iOS or on error, so "no signal" reads as "owns nothing".
     * @returns {Promise<string[]>}
     */
    entitlements: function () {
      if (!available()) return Promise.resolve([]);
      return plugin().entitlements().then(ids, function () { return []; });
    },

    /**
     * Load products by id, with StoreKit's localized prices. Rejects on error so
     * the caller can show a retry rather than a blank or wrong price (A4.8).
     * @param {string[]} productIds
     * @returns {Promise<StoreProduct[]>}
     */
    products: function (productIds) {
      if (!available()) return Promise.reject(new Error('UNAVAILABLE'));
      return plugin().products({ ids: productIds }).then(function (r) {
        return (r && Array.isArray(r.products)) ? r.products : [];
      });
    },

    /** @param {string} id @returns {Promise<PurchaseResult>} */
    purchase: function (id) {
      if (!available()) return Promise.reject(new Error('UNAVAILABLE'));
      return plugin().purchase({ id: id }).then(function (r) {
        return { status: String((r && r.status) || 'unknown'), ids: ids(r) };
      });
    },

    /** Restore Purchases (explicit tap only). @returns {Promise<string[]>} */
    restore: function () {
      if (!available()) return Promise.reject(new Error('UNAVAILABLE'));
      return plugin().restore().then(ids);
    },

    /**
     * Subscribe to entitlement changes pushed by Transaction.updates (Ask to Buy
     * approvals, refunds, Family Sharing). A no-op off iOS.
     * @param {(ids:string[]) => void} cb
     */
    onChange: function (cb) {
      if (!available() || typeof plugin().addListener !== 'function') return;
      plugin().addListener('entitlements', function (r) { cb(ids(r)); });
    },
  };
})();
