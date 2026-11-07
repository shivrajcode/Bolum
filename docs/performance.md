# Dictation latency

Model loading now starts on a separate Rust thread when recording begins. Capture starts
first, so loading does not delay the microphone or block Stop/Cancel. The prepared model
is reused until the existing idle-unload timeout. If recording ends before loading finishes,
only the remaining load time is spent after Stop.

ONNX sessions explicitly disable idle thread spinning. Canary uses up to four CPU threads;
Parakeet retains its eight-thread cap. These separate caps follow local measurements, not
a claim that one thread count is fastest on every CPU. See ONNX Runtime's
[thread-pool guidance](https://onnxruntime.ai/docs/performance/tune-performance/threading.html).

The previous fixed 80 ms pause before Command-V was removed. The clipboard helper already
waits for `pbcopy` to finish. Optional auto-submit retains its separate ordering delay.

## Local measurements

Measured release builds on this Apple Silicon Mac, CPU int8, using the 2.67-second
`tests/fixtures/quick-brown-fox.wav` fixture. All runs recognized the complete sentence.

| Engine | Original load | Original first inference | Canary tuning / first inference |
| --- | ---: | ---: | ---: |
| Canary 180M Flash | 414 ms | 116 ms | 344 ms load / 111 ms inference |
| Parakeet v3 | 1546 ms | 92 ms | Eight-thread cap retained |

The practical change is moving loading into the recording period, rather than making a
warm model dramatically faster. A longer, 13.35-second repeated fixture took roughly
0.9–1.1 seconds with tuned Canary. Longer recordings and different speech can take longer;
these are fixture measurements, not a general accuracy or end-to-end latency guarantee.
Parakeet is the lower-latency choice in these English fixture tests. The selected model
and the user's unload timeout are preserved.

Reproduce load, first-use and repeated inference timings with:

```sh
cargo run --release --example transcribe -- canary-180m-flash tests/fixtures/quick-brown-fox.wav en 5
```

`bolum diagnostics` reports the last inference duration. The application log records audio
duration, inference duration and time from Stop through the delivery attempt, without
logging transcript contents. A successful delivery attempt means input events were sent;
actual insertion still depends on the destination accepting those events.
