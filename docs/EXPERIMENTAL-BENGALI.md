# পরীক্ষামূলক বাংলা VoiceToText — alpha.3

এতে বাংলা কথা থেকে লেখার জন্য native Zipformer engine আছে। অ্যাপ চালাতে Python,
account, API key বা model download লাগে না। এটি stable V1/V2 নয়। ইংরেজি অক্ষরে
ইংরেজি শব্দ রাখা, স্বয়ংক্রিয় যতিচিহ্ন ও decoder dictionary hints এই profile-এ
সমর্থিত নয়। প্রতিটি লেখা দেখে ব্যবহার করুন।

## Windows-এ চালানো

1. Windows-experimental-Bengali-alpha3 artifact-এর সব ফাইল Extract all করুন।
   VoiceToText.exe, bengali-model ও licenses একই ফোল্ডারে রাখুন।
2. Verify.cmd দিয়ে ফাইলগুলোর checksum পরীক্ষা করুন। এটি publisher signature নয়;
   এই alpha unsigned।
3. VoiceToText.exe খুলে Preferences → Import local model-এ
   bengali-model/bengali.vtt-model.json বেছে নিন। Language **বাংলা** রাখুন।
4. Enabled dictionary entries থেকে vocabulary prompt পাঠানো হলে এই profile error
   দেখাবে। এই পরীক্ষামূলক profile-এর জন্য সেগুলো ব্যবহার করবেন না, অথবা অন্য model
   বেছে নিন। Dictionary data নিজে থেকে মুছে ফেলা হয় না।
5. Microphone নির্বাচন করে শুরু / বন্ধ চাপুন, ছোট বাংলা বাক্য বলুন, আবার বন্ধ করুন।
   Result দেখে সংশোধন করে Copy করুন। Windows microphone permission লাগতে পারে।
   Ctrl+Alt+Space দিয়েও শুরু/বন্ধ করা যায়।

এই experimental profile-এ automatic insertion বন্ধ থাকে, Review preference বন্ধ
করলেও। লেখা দেখে explicit Insert বা Copy ব্যবহার করুন। নির্ভরযোগ্য segment timing
নেই বলে SRT বন্ধ; TXT/Markdown ব্যবহার করুন। English/Auto বা vocabulary hint দিলে
দৃশ্যমান error পাবেন।

Windows x64 এবং Microsoft Visual C++ v14 runtime দরকার। DLL/runtime missing হলে
[Microsoft-এর নির্দেশনা](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist)
অনুসরণ করুন। Clean-machine installer, microphone/driver, focus/insertion এবং
accessibility QA এখনো বাকি।

## মাপা ফলের সীমা

Windows native worker-এ ৩০টি development audio-র normalized WER 17.74%,
scalar CER 5.38%, decoder error 0/30। সব ৩০টি লেখা গবেষণা engine-এর ফলের সঙ্গে
হুবহু মিলে গেছে। এগুলো read Indian Bengali; বাংলাদেশি কথ্য,
mixed English, noise, নাম–সংখ্যা বা independent holdout-এর প্রমাণ নয়। Clean WER
লক্ষ্য 10%; এই ফল সেটি পূরণ করেনি। Native app-এর আলাদা ফল ও CI parity evidence
package-এর verification ফোল্ডার এবং
[GitHub integration report](https://github.com/abdullahalmamun-devv/voice-to-text/blob/main/benchmarks/native-bengali-ci/2026-10-18/README.md)-এ দেখুন।

## Developer setup

native/bengali-zipformer-pins.json-এ model ও official sherpa-onnx 1.13.8 CPU
runtime-এর exact hashes আছে। Descriptor arbitrary DLL বা অন্য model bytes
অনুমোদন করতে পারে না; runtime folder-এ extra file-ও গ্রহণ করা হয় না।

Local files দিয়ে prepare-bengali-pack.py pack তৈরি করে। আলাদা
fetch-bengali-test-pack.py শুধু explicit developer/CI setup-এ download করে; app
কখনো সেটি চালায় না। Model এবং sherpa-onnx Apache-2.0, ONNX Runtime MIT। Notices,
upstream third-party notices এবং source URLs pack-এ আছে। Python shipped runtime নয়।

পূর্ণ PRD acceptance, trained/evaluated VAD, bilingual interface, browser adapter,
Windows hardware tests, 200/12 development set এবং independent 1000/50 holdout
এখনো সম্পূর্ণ হয়নি। Linux worker tests Linux desktop support নয়।
