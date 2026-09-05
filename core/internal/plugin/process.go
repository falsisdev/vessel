package plugin

import (
	"bufio"
	"context"
	"errors"
	"fmt"
	"io"
	"log/slog"
	"net"
	"os/exec"
	"strconv"
	"sync"
	"syscall"
	"time"
)

type ProcessState string

const (
	StateStopped  ProcessState = "STOPPED"
	StateStarting ProcessState = "STARTING"
	StateRunning  ProcessState = "RUNNING"
	StateStopping ProcessState = "STOPPING"
	StateCrashed  ProcessState = "CRASHED"
)

var (
	ErrProcessAlreadyRunning = errors.New("process is already running")
	ErrProcessNotRunning     = errors.New("process is not running")
)

type ProcessConfig struct {
	ID             string
	ExecutablePath string
	Args           []string
	Env            []string
	WorkDir        string
}

type Process struct {
	cfg      ProcessConfig
	mu       sync.RWMutex
	state    ProcessState
	port     int
	target   string
	cmd      *exec.Cmd
	exitChan chan struct{}
	exitErr  error

	onCrash func(id string, err error)
}

func NewProcess(cfg ProcessConfig, onCrash func(id string, err error)) *Process {
	return &Process{
		cfg:     cfg,
		state:   StateStopped,
		onCrash: onCrash,
	}
}

func (p *Process) ID() string {
	return p.cfg.ID
}

func (p *Process) State() ProcessState {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.state
}

func (p *Process) Target() string {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.target
}

func (p *Process) Port() int {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.port
}

func (p *Process) Start(ctx context.Context) error {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.state == StateRunning || p.state == StateStarting {
		return ErrProcessAlreadyRunning
	}

	port, err := allocateFreePort()
	if err != nil {
		return fmt.Errorf("failed to allocate port for plugin %s: %w", p.cfg.ID, err)
	}

	p.port = port
	p.target = fmt.Sprintf("127.0.0.1:%d", port)
	p.state = StateStarting

	args := append([]string{}, p.cfg.Args...)
	args = append(args, "-port", strconv.Itoa(port))

	cmd := exec.CommandContext(ctx, p.cfg.ExecutablePath, args...)
	cmd.Dir = p.cfg.WorkDir
	if len(p.cfg.Env) > 0 {
		cmd.Env = p.cfg.Env
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		p.state = StateStopped
		return fmt.Errorf("failed to pipe stdout for %s: %w", p.cfg.ID, err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		p.state = StateStopped
		return fmt.Errorf("failed to pipe stderr for %s: %w", p.cfg.ID, err)
	}

	if err := cmd.Start(); err != nil {
		p.state = StateStopped
		return fmt.Errorf("failed to start plugin process %s: %w", p.cfg.ID, err)
	}

	p.cmd = cmd
	p.state = StateRunning
	p.exitChan = make(chan struct{})

	go p.streamLogs(stdout, "stdout")
	go p.streamLogs(stderr, "stderr")

	go p.monitorProcess(cmd)

	return nil
}

func (p *Process) Stop(timeout time.Duration) error {
	p.mu.Lock()
	if p.state != StateRunning && p.state != StateStarting {
		p.mu.Unlock()
		return nil
	}
	p.state = StateStopping
	cmd := p.cmd
	exitChan := p.exitChan
	p.mu.Unlock()

	if cmd == nil || cmd.Process == nil {
		return nil
	}

	if err := cmd.Process.Signal(syscall.SIGTERM); err != nil {
		_ = cmd.Process.Kill()
	}

	select {
	case <-exitChan:
		return nil
	case <-time.After(timeout):
		_ = cmd.Process.Kill()
		<-exitChan
		return nil
	}
}

func (p *Process) Kill() error {
	p.mu.RLock()
	cmd := p.cmd
	p.mu.RUnlock()

	if cmd == nil || cmd.Process == nil {
		return nil
	}
	return cmd.Process.Kill()
}

func (p *Process) monitorProcess(cmd *exec.Cmd) {
	err := cmd.Wait()

	p.mu.Lock()
	defer p.mu.Unlock()

	p.exitErr = err
	close(p.exitChan)

	if p.state == StateStopping {
		p.state = StateStopped
		return
	}

	p.state = StateCrashed
	slog.Warn("Plugin process crashed or exited unexpectedly",
		"plugin_id", p.cfg.ID,
		"error", err,
	)

	if p.onCrash != nil {
		go p.onCrash(p.cfg.ID, err)
	}
}

func (p *Process) streamLogs(reader io.Reader, pipeName string) {
	scanner := bufio.NewScanner(reader)
	for scanner.Scan() {
		line := scanner.Text()
		slog.Debug("Plugin log output",
			"plugin_id", p.cfg.ID,
			"pipe", pipeName,
			"message", line,
		)
	}
}

func allocateFreePort() (int, error) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return 0, err
	}
	defer listener.Close()

	addr, ok := listener.Addr().(*net.TCPAddr)
	if !ok {
		return 0, errors.New("failed to cast listener address to TCPAddr")
	}
	return addr.Port, nil
}
