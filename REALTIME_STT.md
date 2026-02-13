# Real-Time STT Implementation

## Overview

The STT system has been refactored from a batching approach to true real-time streaming transcription. This eliminates the issues with context reset, state recreation, and lack of incremental decoding.

## Key Changes

### 1. **Persistent State Management** (`service.rs`)

- **Before**: Created a new `WhisperState` for every transcription chunk
- **After**: Single `StreamingTranscriber` instance per WebSocket connection that maintains state throughout the session

### 2. **Rolling Audio Window**

- **Before**: Cleared audio buffer every 1 second (16,000 samples)
- **After**: Maintains a 30-second rolling window of audio context
- Automatically manages window size to prevent memory overflow
- Preserves context for better transcription accuracy

### 3. **Incremental Decoding**

- **Before**: No partial results, only final transcriptions
- **After**:
  - Sends partial results every 500ms
  - Tracks which segments are new vs. previously transcribed
  - Provides both `partial_text` (new content) and `full_text` (complete context)

### 4. **Configuration Constants**

```rust
const SAMPLE_RATE: usize = 16_000;
const WINDOW_SIZE_SECONDS: usize = 30;     // Keep 30 seconds of context
const CHUNK_SIZE_MS: usize = 500;          // Process every 500ms
const CHUNK_SIZE_SAMPLES: usize = 8_000;   // 500ms at 16kHz
const SILENCE_THRESHOLD: f32 = 0.015;
```

## API Changes

### WebSocket Message Types

#### Client → Server

- **Binary**: Audio data (PCM 16-bit, 16kHz, mono)
- **Text "finalize"**: Request final transcription and reset state
- **Text "reset"**: Clear all audio and reset transcriber

#### Server → Client

- **Partial Result**:

  ```json
  {
    "type": "partial",
    "text": "newly transcribed text",
    "full_text": "complete transcription so far"
  }
  ```

- **Final Result**:

  ```json
  {
    "type": "final",
    "text": "complete final transcription"
  }
  ```

- **Reset Confirmation**:
  ```json
  {
    "type": "reset",
    "text": ""
  }
  ```

## How It Works

### Audio Processing Flow

1. Client sends audio chunks via WebSocket (binary messages)
2. Server converts PCM bytes to f32 samples
3. Samples are added to the rolling window
4. When enough new audio accumulates (500ms), transcription runs
5. Server sends partial results with new content
6. Process repeats, maintaining context

### State Management

```rust
pub struct StreamingTranscriber {
    state: WhisperState,              // Persistent Whisper state
    audio_window: Vec<f32>,           // Rolling 30-second window
    last_transcribed_len: usize,      // Track what's been processed
    total_processed: usize,           // Total samples received
}
```

### Key Methods

- `add_audio(&mut self, samples: &[f32])`: Add new audio to rolling window
- `should_transcribe(&self) -> bool`: Check if enough new audio for transcription
- `transcribe_incremental(&mut self) -> TranscriptionResult`: Perform incremental transcription
- `finalize(&mut self) -> TranscriptionResult`: Get final result
- `reset(&mut self)`: Clear all state

## Benefits

### ✅ Fixed Issues

1. **No more context reset** - State persists across chunks
2. **No state recreation** - Single `WhisperState` per connection
3. **Partial tokens** - Real-time partial results every 500ms
4. **Rolling window** - 30 seconds of context maintained
5. **Incremental decoding** - Tracks what's new vs. already transcribed

### 🚀 Performance Improvements

- Reduced latency: Results every 500ms instead of 1 second
- Better accuracy: 30-second context window
- Lower overhead: Reuses Whisper state
- Smoother UX: Partial results show progress

## Usage Example

### Client-Side (JavaScript)

```javascript
const ws = new WebSocket("ws://localhost:3000/stt");

// Send audio
navigator.mediaDevices.getUserMedia({ audio: true }).then((stream) => {
  const audioContext = new AudioContext({ sampleRate: 16000 });
  const source = audioContext.createMediaStreamSource(stream);
  const processor = audioContext.createScriptProcessor(4096, 1, 1);

  processor.onaudioprocess = (e) => {
    const float32 = e.inputBuffer.getChannelData(0);
    const int16 = new Int16Array(float32.length);
    for (let i = 0; i < float32.length; i++) {
      int16[i] = Math.max(-32768, Math.min(32767, float32[i] * 32768));
    }
    ws.send(int16.buffer);
  };

  source.connect(processor);
  processor.connect(audioContext.destination);
});

// Receive results
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);

  if (data.type === "partial") {
    console.log("Partial:", data.text);
    console.log("Full context:", data.full_text);
  } else if (data.type === "final") {
    console.log("Final:", data.text);
  }
};

// Finalize transcription
function finalize() {
  ws.send("finalize");
}

// Reset transcriber
function reset() {
  ws.send("reset");
}
```

## Configuration Tuning

### For Lower Latency

```rust
const CHUNK_SIZE_MS: usize = 250;  // Process every 250ms
```

### For Better Accuracy

```rust
const WINDOW_SIZE_SECONDS: usize = 60;  // Keep 60 seconds of context
```

### For Resource-Constrained Systems

```rust
const WINDOW_SIZE_SECONDS: usize = 15;  // Reduce memory usage
const CHUNK_SIZE_MS: usize = 1000;      // Less frequent processing
```

## Migration Notes

### Breaking Changes

- The old `transcribe()` function has been removed
- WebSocket handler now expects control messages ("finalize", "reset")
- Response format includes both `partial_text` and `full_text`

### Backward Compatibility

If you need the old behavior for specific use cases, you can still batch audio on the client side and send "finalize" after each batch.
