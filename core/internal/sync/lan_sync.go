package sync

import (
	"context"
	"errors"
	"fmt"
	"net"
	"os"
	"runtime"
	"sync"
	"time"
)

var (
	ErrDeviceNotFound = errors.New("target LAN device not found")
)

// Device represents another Vessel instance active on the local area network.
type Device struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	Platform  string    `json:"platform"` // "mac", "linux", "windows", "ios", "android", "tv"
	IP        string    `json:"ip"`
	Port      int       `json:"port"`
	Version   string    `json:"version"`
	LastSeen  time.Time `json:"last_seen"`
}

// RemoteCommand represents a playback or navigation command sent between devices.
type RemoteCommand struct {
	CommandID string  `json:"command_id"`
	Action    string  `json:"action"` // "play", "pause", "seek", "volume", "open_media"
	MediaID   string  `json:"media_id,omitempty"`
	Title     string  `json:"title,omitempty"`
	Position  float64 `json:"position,omitempty"` // seconds
	Volume    float64 `json:"volume,omitempty"`   // 0.0 - 1.0
	SenderID  string  `json:"sender_id"`
	Timestamp int64   `json:"timestamp"`
}

// LANSyncService coordinates local network discovery and remote control.
type LANSyncService struct {
	deviceID   string
	deviceName string
	port       int
	devices    map[string]*Device
	events     chan *RemoteCommand
	mu         sync.RWMutex
	cancel     context.CancelFunc
}

// NewLANSyncService creates a new LANSyncService.
func NewLANSyncService(port int) *LANSyncService {
	hostname, _ := os.Hostname()
	if hostname == "" {
		hostname = "Vessel-Device"
	}

	deviceID := fmt.Sprintf("vessel-%s-%d", hostname, port)

	return &LANSyncService{
		deviceID:   deviceID,
		deviceName: hostname,
		port:       port,
		devices:    make(map[string]*Device),
		events:     make(chan *RemoteCommand, 100),
	}
}

// Start begins periodic LAN broadcasting and peer discovery.
func (s *LANSyncService) Start(ctx context.Context) error {
	ctx, cancel := context.WithCancel(ctx)
	s.cancel = cancel

	// Start simulated LAN beacon & prune loop
	go s.runBeaconLoop(ctx)
	return nil
}

// Stop stops the service.
func (s *LANSyncService) Stop() {
	if s.cancel != nil {
		s.cancel()
	}
}

// RegisterPeer registers or updates a discovered LAN peer.
func (s *LANSyncService) RegisterPeer(dev *Device) {
	s.mu.Lock()
	defer s.mu.Unlock()

	if dev.ID == s.deviceID {
		return
	}
	dev.LastSeen = time.Now().UTC()
	s.devices[dev.ID] = dev
}

// ListDevices returns all currently active LAN devices.
func (s *LANSyncService) ListDevices() []*Device {
	s.mu.RLock()
	defer s.mu.RUnlock()

	var list []*Device
	now := time.Now().UTC()
	for _, d := range s.devices {
		if now.Sub(d.LastSeen) < 30*time.Second {
			list = append(list, d)
		}
	}
	return list
}

// SendCommand dispatches a remote control command to local listeners.
func (s *LANSyncService) SendCommand(cmd *RemoteCommand) error {
	cmd.CommandID = fmt.Sprintf("cmd-%d", time.Now().UnixNano())
	cmd.SenderID = s.deviceID
	cmd.Timestamp = time.Now().Unix()

	select {
	case s.events <- cmd:
		return nil
	default:
		return errors.New("event buffer full")
	}
}

// PollCommand retrieves the next remote command (non-blocking).
func (s *LANSyncService) PollCommand() *RemoteCommand {
	select {
	case cmd := <-s.events:
		return cmd
	default:
		return nil
	}
}

func (s *LANSyncService) runBeaconLoop(ctx context.Context) {
	ticker := time.NewTicker(4 * time.Second)
	defer ticker.Stop()

	// Register self
	s.RegisterPeer(&Device{
		ID:       s.deviceID,
		Name:     s.deviceName,
		Platform: runtime.GOOS,
		IP:       "127.0.0.1",
		Port:     s.port,
		Version:  "1.0.0",
		LastSeen: time.Now().UTC(),
	})

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			s.mu.Lock()
			now := time.Now().UTC()
			for id, d := range s.devices {
				if id != s.deviceID && now.Sub(d.LastSeen) > 45*time.Second {
					delete(s.devices, id)
				}
			}
			s.mu.Unlock()
		}
	}
}

func getOutboundIP() string {
	conn, err := net.Dial("udp", "8.8.8.8:80")
	if err != nil {
		return "127.0.0.1"
	}
	defer conn.Close()
	localAddr := conn.LocalAddr().(*net.UDPAddr)
	return localAddr.IP.String()
}
