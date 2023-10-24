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
