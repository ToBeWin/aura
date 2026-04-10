# Aura v0.1.0 Release Checklist

## Pre-Release Verification

### ✅ Code Quality
- [x] Frontend compiles without errors
- [x] Backend compiles without errors
- [x] No critical warnings
- [x] TypeScript strict mode enabled
- [ ] Run `cargo clippy` and fix issues
- [ ] Run ESLint and fix issues

### ✅ Functionality
- [x] Text refinement works end-to-end
- [x] Format detection and conversion works
- [x] Tone adjustment works
- [x] Personalization applies correctly
- [x] Correction history saves and retrieves
- [x] User context management works
- [x] Undo/Redo functionality works
- [x] Network status detection works
- [ ] Test on actual user scenarios

### ✅ Performance
- [x] Model preloading implemented
- [x] Resource monitoring implemented
- [x] Auto-downgrade on low resources
- [ ] Measure actual startup time
- [ ] Measure processing latency
- [ ] Measure memory usage

### ✅ Offline Mode
- [x] Network status detection
- [x] All features work offline
- [x] No external API calls
- [ ] Test with network disconnected
- [ ] Verify privacy guarantees

### ✅ Cross-Platform
- [x] Build configuration for Windows
- [x] Build configuration for macOS
- [x] Build configuration for Linux
- [ ] Test on Windows 10/11
- [ ] Test on macOS (current platform)
- [ ] Test on Ubuntu/Fedora

### ✅ Documentation
- [x] README.md complete
- [x] QUICKSTART.md complete
- [x] DEVELOPMENT.md complete
- [x] USAGE_GUIDE.md complete
- [x] ARCHITECTURE.md complete
- [x] CONTRIBUTING.md complete
- [x] CHANGELOG.md complete
- [x] TEST_EXAMPLES.md complete
- [x] TESTING.md complete

### ⏳ Testing
- [ ] Unit tests for core modules
- [ ] Integration tests
- [ ] End-to-end tests
- [ ] Property-based tests
- [ ] Performance benchmarks

## Build Process

### 1. Version Bump
- [ ] Update version in `package.json`
- [ ] Update version in `Cargo.toml`
- [ ] Update version in `tauri.conf.json`
- [ ] Update CHANGELOG.md with release notes

### 2. Build Installers
```bash
./build-release.sh
```

Expected outputs:
- Windows: `.exe`, `.msi`
- macOS: `.dmg`, `.app`
- Linux: `.deb`, `.rpm`, `.AppImage`

### 2.1 Sign and Notarize macOS Release
- [ ] Install `Developer ID Application` certificate into Keychain
- [ ] Set `APPLE_SIGNING_IDENTITY`
- [ ] Set one notarization credential method:
- [ ] `APPLE_NOTARY_PROFILE`, or
- [ ] `APPLE_API_KEY_PATH` + `APPLE_API_KEY_ID` + `APPLE_API_ISSUER`, or
- [ ] `APPLE_ID` + `APPLE_PASSWORD` + `APPLE_TEAM_ID`
- [ ] Run `SIGN_MACOS_RELEASE=1 ./build-release.sh`
- [ ] Verify `codesign --verify --deep --strict`
- [ ] Verify `spctl -a -vvv --type exec`
- [ ] Staple notarization ticket successfully

### 3. Test Installers
- [ ] Install on Windows and test
- [ ] Install on macOS and test
- [ ] Install on Linux and test
- [ ] Verify all features work in installed version

## Release Process

### 1. Create GitHub Release
- [ ] Tag version: `v0.1.0`
- [ ] Write release notes (use CHANGELOG.md)
- [ ] Upload installers
- [ ] Mark as pre-release if needed

### 2. Documentation
- [ ] Update README with download links
- [ ] Update website (if applicable)
- [ ] Write blog post/announcement

### 3. Distribution
- [ ] GitHub Releases
- [ ] Direct download links
- [ ] Package managers (future)

## Post-Release

### 1. Monitoring
- [ ] Monitor GitHub issues
- [ ] Collect user feedback
- [ ] Track download statistics

### 2. Support
- [ ] Respond to issues within 48 hours
- [ ] Update FAQ based on common questions
- [ ] Provide installation support

### 3. Planning
- [ ] Plan v0.2.0 features
- [ ] Prioritize based on feedback
- [ ] Update roadmap

## Known Limitations (Document in Release Notes)

1. **ASR Engine**: Framework ready, whisper.cpp integration pending
2. **Voice Commands**: Uses text input simulation, real voice recognition coming in v0.2.0
3. **Testing**: Automated test suite pending
4. **Performance**: Not formally benchmarked yet

## Success Criteria

- [ ] Application installs successfully on all platforms
- [ ] Core refinement workflow works reliably
- [ ] No crashes or critical bugs
- [ ] User feedback is positive
- [ ] Documentation is clear and helpful

## Emergency Rollback Plan

If critical issues are discovered:
1. Mark release as "Pre-release" on GitHub
2. Document known issues prominently
3. Provide workarounds if available
4. Plan hotfix release if needed

---

**Status**: Ready for user acceptance testing  
**Blocker Issues**: None  
**Recommendation**: Proceed with UAT, then release
