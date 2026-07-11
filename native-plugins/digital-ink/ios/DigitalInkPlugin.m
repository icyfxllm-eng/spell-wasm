// Capacitor registration for DigitalInkPlugin — exposes the @objc Swift methods
// to the JS bridge under the plugin name "DigitalInk". STAGED: add to the App
// target alongside DigitalInkPlugin.swift when activating (see README.md).
#import <Foundation/Foundation.h>
#import <Capacitor/Capacitor.h>

CAP_PLUGIN(DigitalInkPlugin, "DigitalInk",
  CAP_PLUGIN_METHOD(downloadModel, CAPPluginReturnPromise);
  CAP_PLUGIN_METHOD(isModelDownloaded, CAPPluginReturnPromise);
  CAP_PLUGIN_METHOD(deleteModel, CAPPluginReturnPromise);
  CAP_PLUGIN_METHOD(recognize, CAPPluginReturnPromise);
)
