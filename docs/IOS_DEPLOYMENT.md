# iOS Deployment Guide

Guide for building and deploying Poziomki to the Apple App Store.

**Prerequisites:**
- Apple Developer account ($99/year) — Required, Poland doesn't qualify for fee waiver
- macOS with Xcode 15+
- D-U-N-S number (for organization account)
- App Store Connect access

---

## 1. Apple Developer Account Setup

### Individual vs Organization
| Type | Cost | Requirements | Recommended For |
|------|------|--------------|-----------------|
| Individual | $99/year | Apple ID | Testing only |
| Organization | $99/year | D-U-N-S, legal entity | Production |

**For Poziomki:** Use Organization account after registering Stowarzyszenie Poziomki.

### D-U-N-S Number
1. Apply at [dnb.com](https://www.dnb.com/duns-number/get-a-duns.html) (free, ~10 business days)
2. Use the registered stowarzyszenie details
3. Wait for Apple to verify (can take additional days)

### Enrollment Steps
1. Go to [developer.apple.com/programs/enroll](https://developer.apple.com/programs/enroll)
2. Sign in with Apple ID
3. Select "Organization"
4. Enter D-U-N-S number
5. Provide legal entity details
6. Pay $99
7. Wait for approval (24-48 hours typical)

---

## 2. App Store Connect Setup

### Create App Record
1. Go to [appstoreconnect.apple.com](https://appstoreconnect.apple.com)
2. My Apps > + > New App
3. Fill in:
   - Platform: iOS
   - Name: Poziomki
   - Primary Language: Polish
   - Bundle ID: app.poziomki.mobile
   - SKU: poziomki-ios-001

### App Information
| Field | Value |
|-------|-------|
| Name | Poziomki |
| Subtitle | Poznaj studentów z Twoich zainteresowań |
| Category | Social Networking |
| Secondary | Lifestyle |
| Age Rating | 17+ (social features) |
| Privacy Policy URL | https://poziomki.app/privacy |

---

## 3. Build Configuration

### Update app.json
```json
{
  "expo": {
    "name": "Poziomki",
    "slug": "poziomki",
    "version": "1.0.0",
    "ios": {
      "bundleIdentifier": "app.poziomki.mobile",
      "buildNumber": "1",
      "supportsTablet": false,
      "infoPlist": {
        "NSCameraUsageDescription": "Poziomki needs camera access to take profile photos",
        "NSPhotoLibraryUsageDescription": "Poziomki needs photo library access to select profile photos",
        "NSPhotoLibraryAddUsageDescription": "Poziomki needs to save photos you download"
      }
    }
  }
}
```

### Required Assets
| Asset | Size | Notes |
|-------|------|-------|
| App Icon | 1024x1024 | No transparency, no rounded corners |
| Screenshots | Various | See Screenshot Requirements below |
| Preview Video | Optional | 15-30 seconds |

### Screenshot Requirements
| Device | Size | Required |
|--------|------|----------|
| iPhone 6.7" | 1290 x 2796 | Yes |
| iPhone 6.5" | 1284 x 2778 | Yes |
| iPhone 5.5" | 1242 x 2208 | Optional |
| iPad Pro 12.9" | 2048 x 2732 | If supporting iPad |

---

## 4. Build Process

### Using EAS Build
```bash
# Install EAS CLI
bun add -g eas-cli

# Login
eas login

# Configure (first time)
eas build:configure

# Build for iOS
eas build --platform ios --profile production
```

### eas.json Configuration
```json
{
  "cli": {
    "version": ">= 3.0.0"
  },
  "build": {
    "development": {
      "ios": {
        "simulator": true
      }
    },
    "preview": {
      "ios": {
        "distribution": "internal"
      }
    },
    "production": {
      "ios": {
        "distribution": "store"
      }
    }
  },
  "submit": {
    "production": {
      "ios": {
        "appleId": "your-apple-id@example.com",
        "ascAppId": "123456789",
        "appleTeamId": "XXXXXXXXXX"
      }
    }
  }
}
```

### Local Build (Alternative)
```bash
# Generate native project
cd apps/mobile
pnpm expo prebuild --platform ios --clean

# Open in Xcode
open ios/Poziomki.xcworkspace

# In Xcode:
# 1. Select your team
# 2. Product > Archive
# 3. Distribute App > App Store Connect
```

---

## 5. Certificates & Provisioning

### Automatic (Recommended)
EAS handles certificates automatically. Just run:
```bash
eas credentials
```

### Manual Setup
1. **Certificates** (developer.apple.com > Certificates)
   - iOS Distribution Certificate
   - Push Notification Certificate (if using push)

2. **Identifiers**
   - App ID: app.poziomki.mobile
   - Enable: Push Notifications, Associated Domains

3. **Provisioning Profiles**
   - App Store Distribution profile

---

## 6. App Store Submission

### Pre-submission Checklist
- [ ] Privacy Policy URL is live and accessible
- [ ] All required screenshots uploaded
- [ ] App description in Polish and English
- [ ] Contact information is valid
- [ ] Test account credentials for review (if needed)
- [ ] Export compliance information

### Submit for Review
```bash
# Using EAS Submit
eas submit --platform ios --profile production

# Or via App Store Connect
# 1. Upload build via Xcode or Transporter
# 2. Select build in App Store Connect
# 3. Fill in version information
# 4. Submit for Review
```

### Review Information
Provide Apple with:
```
Demo Account:
Email: review@poziomki.app
Password: [create test account]

Notes for Review:
- This app requires a Polish university email (.edu.pl) for registration
- Demo account bypasses email verification for review purposes
- App is for Polish university students to connect by shared interests
```

---

## 7. App Review Guidelines

### Common Rejection Reasons

| Issue | Solution |
|-------|----------|
| **4.2 Minimum Functionality** | Ensure app has clear value beyond a website |
| **5.1.1 Data Collection** | Privacy policy must match actual data use |
| **4.3 Spam** | Don't submit multiple similar apps |
| **2.1 App Completeness** | No placeholder content, all features work |
| **5.1.2 Data Use** | Explain why each permission is needed |

### Age Rating
Social apps with user-generated content typically require 17+ rating. Configure in App Store Connect:
- Unrestricted Web Access: Yes (if linking to web)
- User Generated Content: Yes

### Privacy Nutrition Labels
Required fields for App Store:
- Data Linked to You: Email, name, photos, usage data
- Data Used to Track You: None
- Data Not Linked to You: Crash logs

---

## 8. TestFlight (Beta Testing)

### Internal Testing
1. App Store Connect > TestFlight
2. Add internal testers (up to 100)
3. They install via TestFlight app

### External Testing (Requires Review)
1. Create test group
2. Add external testers (up to 10,000)
3. Submit for Beta App Review
4. Usually approved within 24-48 hours

---

## 9. Release Management

### Version Numbering
- `version`: User-visible version (1.0.0, 1.1.0, etc.)
- `buildNumber`: Internal build number (increment each upload)

### Phased Release
Recommended for first release:
1. App Store Connect > App Store > Pricing and Availability
2. Enable "Phased Release for Automatic Updates"
3. Releases to 1%, 2%, 5%, 10%, 20%, 50%, 100% over 7 days

### Expedited Review
For critical bug fixes, request expedited review in App Store Connect. Use sparingly.

---

## 10. Post-Release

### Monitoring
- App Store Connect > Analytics for downloads, crashes
- Monitor App Store reviews
- Check for crash reports in Xcode Organizer

### Updates
1. Increment version and buildNumber
2. Build and submit
3. Add "What's New" text
4. Submit for review

---

## Troubleshooting

### Build Fails
```bash
# Clear caches
cd apps/mobile
rm -rf ios node_modules
pnpm install
pnpm expo prebuild --platform ios --clean
```

### Signing Issues
```bash
# Reset credentials
eas credentials --platform ios
# Select "Remove" and reconfigure
```

### Upload Fails
- Check bundle identifier matches App Store Connect
- Ensure build number is higher than previous uploads
- Verify certificates are valid

---

## Timeline Estimate

| Step | Duration |
|------|----------|
| Developer account approval | 1-3 days |
| First build configuration | 1 day |
| TestFlight testing | 1-2 weeks |
| App Store review | 1-7 days (usually 24-48h) |
| **Total** | **2-4 weeks** |

---

## Resources

- [Apple Developer Documentation](https://developer.apple.com/documentation/)
- [App Store Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
- [Expo iOS Deployment](https://docs.expo.dev/submit/ios/)
- [EAS Build Documentation](https://docs.expo.dev/build/introduction/)

---

*Last updated: 2026-02-03*
