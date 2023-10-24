# Resampler-native-rubato

**This node module is based on rubato, a Rust program that can re-sampler (change sample rate) quickly**

We create a bindign via napi.rs and published it.

## Usage

## Build (if needed)

`yarn && yarn run build`

## Unit Tests

`Cargo test && Yarn test`

### From buffer

```javascript
import { reSampleBuffers } from "@avahq/resampler-native-rubato";

// Check eg inside __test__ folder
```

### From int16Array (slowest function)

```javascript
import { reSampleInt16Array } from "@avahq/resampler-native-rubato";

// Check eg inside __test__ folder
```

### From file

```javascript
import { reSampleAudioFile } from "@avahq/resampler-native-rubato";

// Check eg inside __test__ folder
```

## Release

Ensure you have set your NPM_TOKEN in the GitHub project setting.

In Settings -> Secrets, add NPM_TOKEN into it.

When you want to release the package:

npm version [<newversion> | major | minor | patch | premajor | preminor | prepatch | prerelease [--preid=<prerelease-id>] | from-git]

git push
GitHub actions will do the rest job for you.
