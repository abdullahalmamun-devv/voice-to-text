> Current development: alpha.3 adds an optional experimental native Bengali profile. Read [EXPERIMENTAL-BENGALI.md](docs/EXPERIMENTAL-BENGALI.md) for that package. The general guide below also describes the older alpha.1 archive; no stable model is qualified.

# VoiceToText চালানোর গাইড

**0.2.0-alpha.1 — Windows 11 x64।** V1 ও V2-এর ফিচার একই অ্যাপে আছে। এটি পরীক্ষামূলক alpha; PRD-এর stable release acceptance এখনও সম্পূর্ণ পাস করেনি। Browser-এ সরাসরি লেখা বসানোর adapter এবং বাস্তব Windows/Bangla QA বাকি। লেখা ব্যবহারের আগে যাচাই করুন।

## প্রথমবার চালানো

1. Windows ZIP-এর সব ফাইল একটি স্থায়ী ফোল্ডারে **Extract all** করুন। ZIP-এর ভেতর থেকে EXE চালাবেন না। `models` ফোল্ডারটিও সঙ্গে রাখুন।
2. `Verify.cmd` দিয়ে প্যাকেজের checksum পরীক্ষা করতে পারেন। এটি publisher signature নয়; এই alpha unsigned।
3. `VoiceToText.exe` খুলুন। চাইলে `Install.cmd` চালিয়ে নিজের Windows profile-এ কপি এবং desktop shortcut তৈরি করুন; administrator দরকার নেই।
4. Microphone তালিকা থেকে ডিভাইস বেছে নিন। Windows microphone privacy settings-এ desktop apps-এর অনুমতি চালু থাকতে হবে।
5. **শুরু / বন্ধ** চাপুন, ছোট একটি বাংলা বাক্য বলুন, আবার চাপুন। Result-এ লেখা আসবে। প্রথমবার model load হওয়ায় কিছুটা অপেক্ষা লাগতে পারে।
6. Result সম্পাদনা করে **Copy** করুন, তারপর নিজের পছন্দের textbox-এ paste করুন। অ্যাপ কোনো message নিজে Send করে না।

পুরো প্যাকেজের model স্থানীয়ভাবে পাওয়া যায়; চালানোর সময় internet/account/API key লাগবে না। Model না পাওয়া গেলে Preferences → Import local model থেকে প্যাকেজের `models/ggml-small-q5_1.bin` নির্বাচন করুন।

## Shortcut ও লেখা বসানো

ডিফল্ট shortcut **Ctrl+Alt+Space**। একবার চাপলে recording শুরু, আবার চাপলে বন্ধ। Preferences-এ **Hold shortcut to talk** চালু করে Save preferences দিলে shortcut ধরে কথা বলা এবং ছেড়ে বন্ধ করার পদ্ধতি পাবেন। শুরু/বন্ধ button সবসময় toggle হিসেবে কাজ করে। Shortcut conflict হলে অন্য key/modifier বেছে Save করুন।

Recording চললে আলাদা indicator থাকবে। **Escape / Cancel** দিয়ে বাতিল করুন। প্রতি microphone recording সর্বোচ্চ 120 সেকেন্ড। Windows lock/sleep-এ capture বন্ধ করার code আছে; hardware QA এখনও বাকি। Window-এর X অ্যাপকে tray-তে রাখে। সম্পূর্ণ বন্ধ করতে tray menu → **Exit** ব্যবহার করুন। Recording-এর সময় tray menu খুললে capture বাতিল হয়।

Review ডিফল্টে চালু: ফল আসলে app-এ দেখুন। **Insert…** চাপুন, intended native text field-এ cursor রাখুন, shortcut চাপুন এবং সব modifier ছেড়ে দিন। সমর্থিত field হলে app যাচাই করে লেখা বসানোর চেষ্টা করে। Focus, selection, caret বা content বদলে গেলে result ধরে রাখে।

এই alpha-তে native Unicode Edit/RichEdit controls-এর জন্য insertion code আছে। Chrome/Edge/Firefox textboxes, Word, terminal, elevated apps, custom editors এবং অচেনা controls-এর ক্ষেত্রে **Copy করে নিজে paste করুন**। বাস্তব Windows compatibility matrix এখনও যাচাই হয়নি।

**Undo…** কেবল app-এর আগের যাচাইকৃত insertion-এর জন্য: original field-এ ফিরে shortcut দিন। এর পরে অন্য লেখা বা caret বদলালে Undo বন্ধ থাকবে। Unconfirmed delivery দেখালে প্রথমে target দেখুন; blind retry করলে duplicate হতে পারে।

Review বন্ধ করলে shortcut দিয়ে recording শুরুর সময়কার unchanged supported field-এ automatic insertion চেষ্টা করবে। এই alpha ব্যবহার শুরুর সময় Review চালু রাখাই ব্যবহারযোগ্য baseline।

## ভাষা, সংখ্যা ও Dictionary

- Preferences → Language: বাংলা, English অথবা বাংলা + English। শেষটি automatic language detection; প্রতিটি code-switch ঠিক থাকবে এমন নিশ্চয়তা নেই।
- Number style: recognized spelling, বাংলা সংখ্যা অথবা Latin digits। নির্দিষ্ট সম্পূর্ণ integer amount phrase সমর্থিত; অস্পষ্ট digit sequence/range/relative date আন্দাজ করে বদলায় না।
- Dictionary & snippets → Preferred spelling দিন। Spoken alias ফাঁকা রাখলে preferred term শুধু decoding context-এ যায়; alias দিলে exact phrase replacement হয়। **Add / update** দিয়ে নিশ্চিত করুন। সর্বোচ্চ 500 entry।
- Result-এ ছোট ভুল ঠিক করে **Save current correction** চাপলে একটি দৃশ্যমান exact-phrase rule প্রস্তাব হয়। **Add / update** না চাপা পর্যন্ত তা শেখানো/সংরক্ষণ হয় না। Hidden training হয় না।

Dictionary recognition-এ সাহায্য করতে পারে, কিন্তু নাম/শব্দের নিখুঁত recognition নিশ্চিত করে না।

## V2 ফিচার

Preferences থেকে পৃথকভাবে চালু করুন:

- **Live draft preview:** আনুমানিক প্রতি পাঁচ সেকেন্ডে app-এর result-এ খসড়া; external editor-এ draft লেখা হয় না। Experimental এবং CPU-র উপর চাপ বাড়াতে পারে।
- **Save local transcript history:** শুধু কথার লেখা, raw audio নয়; 7/30 দিন বা manual retention। সর্বোচ্চ 500 entry। Local history → Open/Copy/Export/Delete all। বন্ধ করলে পুরোনো history মুছে যায় না; Delete all ব্যবহার করুন। Expiry app history পড়া/নতুন entry লেখার সময় কার্যকর হয়।
- **Explicit formatting / snippet commands:** পুরো utterance `নতুন লাইন` অথবা `নতুন অনুচ্ছেদ` হলে formatting; snippet-এর জন্য `স্নিপেট <trigger>` বলুন। অন্য বাক্যের মাঝের শব্দগুলো command হয় না।
- `শেষ শব্দ মুছুন` / `শেষ লাইন মুছুন`: app-এর আগের result-এর সীমিত সম্পাদনা; অন্য অ্যাপের লেখা মুছে না, automatic insertion-ও করে না।
- **Open WAV:** uncompressed PCM/float WAV, সর্বোচ্চ 10 মিনিট/250 MB; MP3/video/batch import নেই। বড় file পড়ার সময় UI কিছুটা থামতে পারে।
- **Export TXT / Markdown:** বর্তমান edited result। **SRT (raw):** engine-এর মূল timed segments; dictionary formatting/result edits এতে থাকে না। অবিশ্বস্ত timing হলে SRT export বন্ধ হয়।

## নিজের data ও recovery

Settings ও dictionary: `%LOCALAPPDATA%\VoiceToText\settings.json`; আগের valid JSON: `settings.backup.json`। History চালালে একই ফোল্ডারে `history.json`। এই ফাইলগুলো encrypted নয়। History বন্ধ থাকা অবস্থায় নতুন speech transcript disk-এ লেখা হয় না; settings/dictionary আলাদা local files। OS clipboard history, paging, backups ও অন্য অ্যাপের সংরক্ষণ app-এর নিয়ন্ত্রণে নেই।

Preferences থেকে settings/dictionary export করে নিজের backup রাখতে পারেন। Settings import-এর পরে tray → Exit করে পুনরায় খুলুন। Corrupt settings পেলে app reset না করে error দেখাবে; Exit করে primary file নিরাপদে সরিয়ে backup copy-কে `settings.json` করুন। Model checksum error হলে original package থেকে model+manifest পুনরায় copy করুন। Save failed দেখালে সেই পরিবর্তন শুধু চলতি memory-তে প্রযোজ্য; save retry বা restart করে disk settings-এ ফিরুন।

Update/rollback করতে app tray থেকে Exit করে আগের/নতুন full package আলাদা folder-এ রাখুন; optional installer দিয়ে local files replace করতে পারেন। Model path বদলে গেলে Import local model আবার ব্যবহার করুন। Installer/upgrade/rollback Windows-এ এখনও বাস্তব পরীক্ষা করা হয়নি। Uninstall করতে startup option off ও Save, tray Exit, install folder ও shortcut মুছুন। Settings/history মুছবেন কি না নিজে নির্বাচন করুন।

**বাস্তব সীমা:** বাংলা accuracy benchmark, Windows microphone/insertion safety suite, screen reader/DPI/theme checks ও resource/network capture এখনও pending। সম্পূর্ণ PRD status এবং manual test steps `docs/REQUIREMENTS-STATUS.md` ও `docs/WINDOWS-QA.md`-তে আছে।
