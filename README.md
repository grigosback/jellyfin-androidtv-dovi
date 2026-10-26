<h1 align="center">RemuxDV</h1>
<h3 align="center">An unofficial Jellyfin client for Android TV that plays Dolby Vision profile 7 remuxes</h3>

---

RemuxDV is a fork of [Jellyfin for Android TV](https://github.com/jellyfin/jellyfin-androidtv). It is not
affiliated with or endorsed by the Jellyfin project or Dolby Laboratories.

UHD Blu-ray remuxes use Dolby Vision profile 7 (a base layer plus an enhancement layer). Most Android TV
Dolby Vision decoders only support single layer profiles, so the official app makes the server remux these
files and strip the Dolby Vision metadata, leaving plain HDR10. RemuxDV converts them on the device instead,
the same way Kodi does with "convert Dolby Vision":

- every Dolby Vision RPU is converted to profile 8.1 with [libdovi](https://github.com/quietvoid/dovi_tool)
  (mode 2) and the enhancement layer is dropped, right before each frame reaches the decoder
- the server direct plays the file as is, so there is no remux or transcode
- HDR10+ metadata of hybrid releases is removed while the Dolby Vision decoder is used, so TVs that prefer
  HDR10+ stay in Dolby Vision mode
- DTS and TrueHD audio are decoded on the device when they can't be passed through, instead of triggering a
  server remux

Devices with a native profile 7 decoder keep the original stream. A full enhancement layer (FEL) is
discarded, as with Kodi; minimal enhancement layer (MEL) files lose nothing.

### Install

Download the APK from [Releases](https://github.com/grigosback/jellyfin-androidtv-dovi/releases) and sideload
it (for example with adb or an app like "Send Files to TV"). It installs next to the official app.

### Build

- Dolby Vision conversion library (Rust, needs rustup with the Android targets, cargo-ndk and the NDK):
  `dovi-jni/build-android.sh`. Prebuilt libraries are committed.
- App: `./gradlew :app:assembleDebug`, or `scripts/build-release.sh <version>` for a signed release.
- Branding: `python3 branding/generate.py` regenerates the icon, banner and logo.

The rest of this file is the upstream README.

---

<h1 align="center">Jellyfin for Android TV</h1>
<h3 align="center">Part of the <a href="https://jellyfin.org">Jellyfin Project</a></h3>

---

<p align="center">
<img alt="Logo banner" src="https://raw.githubusercontent.com/jellyfin/jellyfin-ux/master/branding/SVG/banner-logo-solid.svg?sanitize=true"/>
<br/><br/>
<a href="https://github.com/jellyfin/jellyfin-androidtv">
<img alt="GPL 2.0 License" src="https://img.shields.io/github/license/jellyfin/jellyfin-androidtv.svg"/>
</a>
<a href="https://github.com/jellyfin/jellyfin-androidtv/releases">
<img alt="Current Release" src="https://img.shields.io/github/release/jellyfin/jellyfin-androidtv.svg"/>
</a>
<a href="https://translate.jellyfin.org/projects/jellyfin-android/jellyfin-androidtv/">
<img alt="Translation Status" src="https://translate.jellyfin.org/widgets/jellyfin-android/-/jellyfin-androidtv/svg-badge.svg"/>
</a>
<br/>
<a href="https://opencollective.com/jellyfin">
<img alt="Donate" src="https://img.shields.io/opencollective/all/jellyfin.svg?label=backers"/>
</a>
<a href="https://features.jellyfin.org">
<img alt="Feature Requests" src="https://img.shields.io/badge/fider-vote%20on%20features-success.svg"/>
</a>
<a href="https://matrix.to/#/+jellyfin:matrix.org">
<img alt="Chat on Matrix" src="https://img.shields.io/matrix/jellyfin:matrix.org.svg?logo=matrix"/>
</a>
<br/>
<a href="https://play.google.com/store/apps/details?id=org.jellyfin.androidtv">
<img width="153" alt="Jellyfin on Google Play" src="https://jellyfin.org/images/store-icons/google-play.png"/>
</a>
<a href="https://www.amazon.com/gp/aw/d/B07TX7Z725">
<img width="153" alt="Jellyfin on Amazon Appstore" src="https://jellyfin.org/images/store-icons/amazon.png"/>
</a>
<a href="https://f-droid.org/en/packages/org.jellyfin.androidtv/">
<img width="153" alt="Jellyfin on F-Droid" src="https://jellyfin.org/images/store-icons/fdroid.png"/>
</a>
<br/>
<a href="https://repo.jellyfin.org/releases/client/androidtv/">Download archive</a>
</p>

Jellyfin for Android TV is a Jellyfin client for Android TV, Nvidia Shield, and Amazon Fire TV devices. We welcome all contributions and pull
requests! If you have a larger feature in mind please open an issue so we can discuss the implementation before you start. 

## Building

The app uses Gradle and requires the Android SDK. We recommend using Android Studio, which includes all required dependencies, for
development and building. For manual building without Android Studio make sure a compatible JDK and Android SDK are installed and in your
PATH, then use the Gradle wrapper (`./gradlew`) to build the project with the `assembleDebug` Gradle task to generate an apk file:

```shell
./gradlew assembleDebug
```

The task will create an APK file in the `/app/build/outputs/apk/debug` directory. This APK file uses a different app-id from our stable
builds and can be manually installed to your device.

## Branching

The `master` branch is the primary development branch and the target for all pull requests. It is **unstable** and may contain breaking
changes or unresolved bugs. For production deployments and forks, always use the latest `release-x.y.z` branch. Do not base production work
or long-lived forks on `master`.

Release branches are created at the start of a beta cycle and are kept up to date with each published release. Maintainers will cherry-pick
selected changes into release branches as needed for backports. These branches are reused for subsequent patch releases.

## Translating

Translations can be improved very easily from our [Weblate](https://translate.jellyfin.org/projects/jellyfin-android/jellyfin-androidtv)
instance. Look through the following graphic to see if your native language could use some work! We cannot accept changes to translation
files via pull requests.

<p align="center">
<a href="https://translate.jellyfin.org/engage/jellyfin-android/">
<img alt="Detailed Translation Status" src="https://translate.jellyfin.org/widgets/jellyfin-android/-/jellyfin-androidtv/multi-auto.svg"/>
</a>
</p>
