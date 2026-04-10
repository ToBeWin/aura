import Foundation
import Speech

enum TranscriptionError: Error {
    case invalidArguments
    case recognizerUnavailable
    case authorizationDenied(String)
    case recognitionFailed(String)
}

var debugLogPath: String?

func appendDebug(_ message: String) {
    guard let debugLogPath else { return }
    let line = "\(ISO8601DateFormatter().string(from: Date())) \(message)\n"
    if let data = line.data(using: .utf8) {
        if FileManager.default.fileExists(atPath: debugLogPath) {
            if let handle = try? FileHandle(forWritingTo: URL(fileURLWithPath: debugLogPath)) {
                try? handle.seekToEnd()
                try? handle.write(contentsOf: data)
                try? handle.close()
            }
        } else {
            FileManager.default.createFile(atPath: debugLogPath, contents: data)
        }
    }
}

func waitUntil(timeout: TimeInterval, condition: @escaping () -> Bool) -> Bool {
    let deadline = Date().addingTimeInterval(timeout)
    while !condition() && Date() < deadline {
        RunLoop.current.run(mode: .default, before: Date().addingTimeInterval(0.05))
    }
    return condition()
}

func waitForAuthorization() -> SFSpeechRecognizerAuthorizationStatus {
    var resolvedStatus: SFSpeechRecognizerAuthorizationStatus = .notDetermined
    var didResolve = false

    appendDebug("requesting speech authorization")
    SFSpeechRecognizer.requestAuthorization { status in
        resolvedStatus = status
        didResolve = true
        appendDebug("authorization callback: \(status.rawValue)")
    }

    if !waitUntil(timeout: 10) { didResolve } {
        appendDebug("authorization wait timed out")
    }
    return resolvedStatus
}

func authorizationMessage(for status: SFSpeechRecognizerAuthorizationStatus) -> String {
    switch status {
    case .authorized:
        return "authorized"
    case .denied:
        return "speech recognition permission denied"
    case .restricted:
        return "speech recognition is restricted on this device"
    case .notDetermined:
        return "speech recognition permission not determined"
    @unknown default:
        return "speech recognition authorization status is unknown"
    }
}

func transcribeAudio(at audioPath: String, language: String) throws -> String {
    appendDebug("starting transcription for \(audioPath) with language \(language)")
    let status = waitForAuthorization()
    guard status == .authorized else {
        throw TranscriptionError.authorizationDenied(authorizationMessage(for: status))
    }
    appendDebug("authorization granted")

    let locale = Locale(identifier: language)
    guard let recognizer = SFSpeechRecognizer(locale: locale) ?? SFSpeechRecognizer() else {
        throw TranscriptionError.recognizerUnavailable
    }

    guard recognizer.isAvailable else {
        throw TranscriptionError.recognizerUnavailable
    }
    appendDebug("speech recognizer available")

    let request = SFSpeechURLRecognitionRequest(url: URL(fileURLWithPath: audioPath))
    request.shouldReportPartialResults = false
    if #available(macOS 13.0, *) {
        request.requiresOnDeviceRecognition = false
        request.addsPunctuation = true
    }

    var resolvedText = ""
    var resolvedError: String?
    var didFinish = false

    var recognitionTask: SFSpeechRecognitionTask?
    appendDebug("starting recognition task")
    recognitionTask = recognizer.recognitionTask(with: request) { result, error in
        if let result {
            resolvedText = result.bestTranscription.formattedString.trimmingCharacters(in: .whitespacesAndNewlines)
            appendDebug("recognition callback result, final=\(result.isFinal), text=\(resolvedText)")
            if result.isFinal {
                recognitionTask?.cancel()
                didFinish = true
            }
            return
        }

        if let error {
            resolvedError = error.localizedDescription
            appendDebug("recognition callback error: \(resolvedError ?? "unknown")")
            recognitionTask?.cancel()
            didFinish = true
        }
    }

    let waitSucceeded = waitUntil(timeout: 120) { didFinish }
    recognitionTask?.cancel()

    if !waitSucceeded {
        appendDebug("recognition wait timed out")
        throw TranscriptionError.recognitionFailed("speech recognition timed out")
    }

    if let resolvedError {
        throw TranscriptionError.recognitionFailed(resolvedError)
    }

    if resolvedText.isEmpty {
        throw TranscriptionError.recognitionFailed("speech recognition returned empty text")
    }

    return resolvedText
}

do {
    let arguments = CommandLine.arguments
    guard arguments.count >= 3 else {
        throw TranscriptionError.invalidArguments
    }

    debugLogPath = ProcessInfo.processInfo.environment["AURA_SPEECH_DEBUG_LOG"]
    appendDebug("helper launched with \(arguments.count) args")
    let text = try transcribeAudio(at: arguments[1], language: arguments[2])
    if arguments.count >= 4 {
        let outputURL = URL(fileURLWithPath: arguments[3])
        try text.write(to: outputURL, atomically: true, encoding: .utf8)
    } else {
        FileHandle.standardOutput.write(Data(text.utf8))
    }
} catch TranscriptionError.invalidArguments {
    FileHandle.standardError.write(Data("Usage: transcribe_audio.swift <audio_path> <language> [output_path]\n".utf8))
    exit(64)
} catch TranscriptionError.recognizerUnavailable {
    FileHandle.standardError.write(Data("Speech recognizer is unavailable on this machine.\n".utf8))
    exit(65)
} catch TranscriptionError.authorizationDenied(let message) {
    FileHandle.standardError.write(Data("\(message)\n".utf8))
    exit(66)
} catch TranscriptionError.recognitionFailed(let message) {
    FileHandle.standardError.write(Data("\(message)\n".utf8))
    exit(67)
} catch {
    FileHandle.standardError.write(Data("\(error.localizedDescription)\n".utf8))
    exit(68)
}
