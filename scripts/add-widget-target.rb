#!/usr/bin/env ruby
# frozen_string_literal: true
#
# F3 — add the `SpellWidgets` WidgetKit app-extension target to App.xcodeproj.
#
# Idempotent: re-running detects the existing target and exits without touching
# the project. Uses the `xcodeproj` gem (already a fastlane dependency). Run:
#
#   ruby scripts/add-widget-target.rb
#
# Provisioning (App Group capability + the extension's signing profile) is NOT
# something a project-file edit can grant — see the PR for the exact Apple portal
# steps Eric must take before any TestFlight build.

require 'xcodeproj'

PROJECT = File.expand_path('../ios/App/App.xcodeproj', __dir__)
APP_BUNDLE_ID = 'net.spellgame.app'
WIDGET_NAME = 'SpellWidgets'
WIDGET_BUNDLE_ID = "#{APP_BUNDLE_ID}.#{WIDGET_NAME}"
APP_GROUP = 'group.net.spellgame.app'
TEAM = 'WCH6H5NAWH'
DEPLOYMENT = '15.0'

project = Xcodeproj::Project.open(PROJECT)

app_target = project.targets.find { |t| t.name == 'App' }
raise 'App target not found' unless app_target

if project.targets.any? { |t| t.name == WIDGET_NAME }
  puts "[skip] target #{WIDGET_NAME} already exists — nothing to do."
  exit 0
end

# --- 1. the app-extension target -------------------------------------------
widget = project.new_target(
  :app_extension, WIDGET_NAME, :ios, DEPLOYMENT, project.products_group, :swift
)

# --- 2. source files (a group mirroring ios/App/SpellWidgets) ---------------
group = project.main_group.find_subpath('SpellWidgets', true)
group.set_source_tree('SOURCE_ROOT')
group.set_path('SpellWidgets')

widget_sources = %w[
  SpellWidgetBundle.swift
  StreakWidget.swift
  DailyWidget.swift
  WidgetStrings.swift
  WidgetSupport.swift
]
widget_sources.each do |name|
  ref = group.new_reference(name)
  widget.source_build_phase.add_file_reference(ref, true)
end

# Shared App Group contract — compiled into BOTH the widget and the App target.
shared_group = group.find_subpath('Shared', true)
shared_group.set_source_tree('SOURCE_ROOT')
shared_group.set_path('SpellWidgets/Shared')
shared_ref = shared_group.new_reference('WidgetSharedState.swift')
widget.source_build_phase.add_file_reference(shared_ref, true)
app_target.source_build_phase.add_file_reference(shared_ref, true)

# Info.plist + entitlements as plain file references (never in a build phase).
group.new_reference('Info.plist')
group.new_reference('SpellWidgets.entitlements')

# --- 3. system frameworks ---------------------------------------------------
widget.add_system_framework('WidgetKit')
widget.add_system_framework('SwiftUI')

# --- 4. build settings on both configs --------------------------------------
widget.build_configurations.each do |config|
  s = config.build_settings
  s['PRODUCT_BUNDLE_IDENTIFIER'] = WIDGET_BUNDLE_ID
  s['PRODUCT_NAME'] = '$(TARGET_NAME)'
  s['INFOPLIST_FILE'] = 'SpellWidgets/Info.plist'
  s['CODE_SIGN_ENTITLEMENTS'] = 'SpellWidgets/SpellWidgets.entitlements'
  s['GENERATE_INFOPLIST_FILE'] = 'NO'
  s['SWIFT_VERSION'] = '5.0'
  s['IPHONEOS_DEPLOYMENT_TARGET'] = DEPLOYMENT
  s['TARGETED_DEVICE_FAMILY'] = '1,2'
  s['MARKETING_VERSION'] = '1.1'
  s['CURRENT_PROJECT_VERSION'] = '47' # match the app; NO build bump (build 48 in review)
  s['DEVELOPMENT_TEAM'] = TEAM
  s['CODE_SIGN_STYLE'] = 'Automatic'
  s['SKIP_INSTALL'] = 'NO'
  s['LD_RUNPATH_SEARCH_PATHS'] = ['$(inherited)', '@executable_path/Frameworks',
                                  '@executable_path/../../Frameworks']
  s['ASSETCATALOG_COMPILER_GLOBAL_ACCENT_COLOR_NAME'] = 'AccentColor'
  s['ASSETCATALOG_COMPILER_WIDGET_BACKGROUND_COLOR_NAME'] = 'WidgetBackground'
end

# --- 5. App target: App Group entitlement + embed the extension -------------
app_target.build_configurations.each do |config|
  config.build_settings['CODE_SIGN_ENTITLEMENTS'] = 'App/App.entitlements'
end
# reference the app entitlements in the project navigator
app_group = project.main_group.find_subpath('App', true)
unless app_group.files.any? { |f| f.display_name == 'App.entitlements' }
  app_group.new_reference('App.entitlements')
end

app_target.add_dependency(widget)

embed = app_target.new_copy_files_build_phase('Embed Foundation Extensions')
embed.symbol_dst_subfolder_spec = :plug_ins
embed_file = embed.add_file_reference(widget.product_reference, true)
embed_file.settings = { 'ATTRIBUTES' => ['RemoveHeadersOnCopy'] }

project.save
puts "[ok] added #{WIDGET_NAME} (#{WIDGET_BUNDLE_ID}); App now embeds it."
puts "     App Group: #{APP_GROUP}"
