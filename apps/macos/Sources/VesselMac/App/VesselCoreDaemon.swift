import Foundation
import Combine

public class VesselCoreDaemon: ObservableObject {
    public static let shared = VesselCoreDaemon()

    @Published public var isCoreRunning: Bool = false
    @Published public var coreStatusText: String = "Connecting to Vessel Core..."

    private var process: Process?
    private var healthCheckTimer: Timer?
    public var baseURL: URL { URL(string: UserDefaults.standard.string(forKey: "vessel_api_base_url") ?? "http://127.0.0.1:8080")! }

    private init() {
        startHealthCheck()
    }

    public func startHealthCheck() {
        checkHealth { [weak self] isHealthy in
            guard let self = self else { return }
            if isHealthy {
                self.isCoreRunning = true
                self.coreStatusText = "Connected to Vessel Core (Port 8080)"
            } else {
                self.launchCoreProcessIfNeeded()
            }
        }

        healthCheckTimer?.invalidate()
        healthCheckTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { [weak self] _ in
            self?.checkHealth { isHealthy in
                DispatchQueue.main.async {
                    self?.isCoreRunning = isHealthy
                    if isHealthy {
                        self?.coreStatusText = "Connected to Vessel Core (Port 8080)"
                    } else {
                        self?.coreStatusText = "Connecting..."
                    }
                }
            }
        }
    }

    public func checkHealth(completion: @escaping (Bool) -> Void) {
        let url = baseURL.appendingPathComponent("api/plugins")
        var request = URLRequest(url: url)
        request.timeoutInterval = 2.0

        URLSession.shared.dataTask(with: request) { _, response, error in
            let success = (error == nil) && ((response as? HTTPURLResponse)?.statusCode == 200)
            DispatchQueue.main.async {
                completion(success)
            }
        }.resume()
    }

    private func launchCoreProcessIfNeeded() {
        if isCoreRunning { return }

        // Look for binary in current directory or parent directory
        let fileManager = FileManager.default
        let currentDir = fileManager.currentDirectoryPath
        let userPath = UserDefaults.standard.string(forKey: "vessel_core_binary_path")
        let possiblePaths = [
            userPath ?? "",
            "\(currentDir)/bin/vessel",
            "\(currentDir)/../bin/vessel",
            "\(currentDir)/../../bin/vessel",
            Bundle.main.path(forResource: "vessel", ofType: nil) ?? ""
        ]

        guard let executablePath = possiblePaths.first(where: { fileManager.isExecutableFile(atPath: $0) }) else {
            DispatchQueue.main.async {
                self.coreStatusText = "Waiting for external daemon on :8080"
            }
            return
        }

        let p = Process()
        p.executableURL = URL(fileURLWithPath: executablePath)
        let host = UserDefaults.standard.string(forKey: "vessel_core_host") ?? "127.0.0.1"
        let port = UserDefaults.standard.string(forKey: "vessel_core_port") ?? "8080"
        p.arguments = ["serve", "-port", port, "-host", host]

        let pipe = Pipe()
        p.standardOutput = pipe
        p.standardError = pipe

        do {
            try p.run()
            self.process = p
            let apiBase = "http://\(host):\(port)"
            UserDefaults.standard.set(apiBase, forKey: "vessel_api_base_url")
            DispatchQueue.main.async {
                self.coreStatusText = "Spawned Vessel Core daemon"
            }
        } catch {
            DispatchQueue.main.async {
                self.coreStatusText = "Failed to spawn daemon: \(error.localizedDescription)"
            }
        }
    }

    public func shutdown() {
        healthCheckTimer?.invalidate()
        if let p = process, p.isRunning {
            p.terminate()
        }
        process = nil
    }

    deinit {
        shutdown()
    }
}
