package service

import (
	"archive/tar"
	"archive/zip"
	"bytes"
	"compress/gzip"
	"encoding/xml"
	"errors"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
	"unicode"

	"github.com/falsisdev/vessel/core/internal/domain/reading"
)

type LocalBookSession struct {
	ID          string         `json:"id"`
	FileName    string         `json:"file_name"`
	Title       string         `json:"title"`
	Format      string         `json:"format"` // "pdf", "text", "archive"
	TotalPages  int            `json:"total_pages"`
	TextContent string         `json:"text_content,omitempty"`
	PDFURL      string         `json:"pdf_url,omitempty"`
	Pages       []reading.Page `json:"pages,omitempty"`
	PageFiles   map[int]string `json:"-"`
	TempDir     string         `json:"-"`
	CreatedAt   time.Time      `json:"created_at"`
}

type LocalReaderService struct {
	sessions map[string]*LocalBookSession
	tempDir  string
	mu       sync.RWMutex
}

func NewLocalReaderService(baseTempDir string) *LocalReaderService {
	if baseTempDir == "" {
		home, _ := os.UserHomeDir()
		baseTempDir = filepath.Join(home, ".vessel", "temp_reader")
	}
	_ = os.MkdirAll(baseTempDir, 0755)

	return &LocalReaderService{
		sessions: make(map[string]*LocalBookSession),
		tempDir:  baseTempDir,
	}
}

// OpenFile processes a local file path or uploaded file and returns a ready-to-read session
func (s *LocalReaderService) OpenFile(filePath string) (*LocalBookSession, error) {
	info, err := os.Stat(filePath)
	if err != nil {
		return nil, fmt.Errorf("file not found: %w", err)
	}

	sessionID := fmt.Sprintf("local_%d", time.Now().UnixNano())
	workDir := filepath.Join(s.tempDir, sessionID)
	_ = os.MkdirAll(workDir, 0755)

	fileName := info.Name()
	ext := strings.ToLower(filepath.Ext(fileName))
	baseName := strings.TrimSuffix(fileName, filepath.Ext(fileName))

	session := &LocalBookSession{
		ID:        sessionID,
		FileName:  fileName,
		Title:     baseName,
		TempDir:   workDir,
		PageFiles: make(map[int]string),
		CreatedAt: time.Now().UTC(),
	}

	switch ext {
	case ".pdf":
		session.Format = "pdf"
		destPDF := filepath.Join(workDir, "document.pdf")
		if err := copyFile(filePath, destPDF); err == nil {
			session.PDFURL = fmt.Sprintf("/api/reading/local/file?id=%s&page=0", sessionID)
		} else {
			return nil, fmt.Errorf("failed to prepare PDF: %w", err)
		}

	case ".txt":
		session.Format = "text"
		data, err := os.ReadFile(filePath)
		if err != nil {
			return nil, err
		}
		session.TextContent = string(data)

	case ".md", ".markdown":
		session.Format = "text"
		data, err := os.ReadFile(filePath)
		if err != nil {
			return nil, err
		}
		session.TextContent = string(data)

	case ".docx":
		session.Format = "text"
		text, err := extractDocxText(filePath)
		if err != nil {
			return nil, fmt.Errorf("failed to parse DOCX: %w", err)
		}
		session.TextContent = text

	case ".cbz", ".zip":
		session.Format = "archive"
		if err := s.extractZipArchive(filePath, workDir, session); err != nil {
			return nil, err
		}

	case ".tar", ".tgz", ".tar.gz":
		session.Format = "archive"
		if err := s.extractTarArchive(filePath, workDir, session); err != nil {
			return nil, err
		}

	case ".cbr", ".rar", ".7z":
		session.Format = "archive"
		if err := s.extractExternalArchive(filePath, workDir, session); err != nil {
			return nil, err
		}

	default:
		return nil, fmt.Errorf("unsupported book format: %s", ext)
	}

	s.mu.Lock()
	s.sessions[sessionID] = session
	s.mu.Unlock()

	return session, nil
}

// GetSession returns an active local reading session
func (s *LocalReaderService) GetSession(sessionID string) (*LocalBookSession, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	sess, ok := s.sessions[sessionID]
	return sess, ok
}

// ServeFile serves a page image or PDF for a local reading session
func (s *LocalReaderService) ServeFile(w http.ResponseWriter, r *http.Request, sessionID string, pageNum int) {
	s.mu.RLock()
	sess, ok := s.sessions[sessionID]
	s.mu.RUnlock()

	if !ok {
		http.Error(w, "Session not found", http.StatusNotFound)
		return
	}

	if sess.Format == "pdf" && pageNum == 0 {
		pdfPath := filepath.Join(sess.TempDir, "document.pdf")
		w.Header().Set("Content-Type", "application/pdf")
		http.ServeFile(w, r, pdfPath)
		return
	}

	filePath, hasPage := sess.PageFiles[pageNum]
	if !hasPage {
		http.Error(w, "Page not found", http.StatusNotFound)
		return
	}

	ext := strings.ToLower(filepath.Ext(filePath))
	switch ext {
	case ".png":
		w.Header().Set("Content-Type", "image/png")
	case ".webp":
		w.Header().Set("Content-Type", "image/webp")
	case ".gif":
		w.Header().Set("Content-Type", "image/gif")
	default:
		w.Header().Set("Content-Type", "image/jpeg")
	}
	w.Header().Set("Cache-Control", "public, max-age=86400")
	http.ServeFile(w, r, filePath)
}

func (s *LocalReaderService) extractZipArchive(srcZip, destDir string, sess *LocalBookSession) error {
	r, err := zip.OpenReader(srcZip)
	if err != nil {
		return fmt.Errorf("failed to open zip/cbz: %w", err)
	}
	defer r.Close()

	var imgFiles []string
	for _, f := range r.File {
		if f.FileInfo().IsDir() {
			continue
		}
		if isImageFile(f.Name) {
			destPath := filepath.Join(destDir, filepath.Base(f.Name))
			rc, err := f.Open()
			if err == nil {
				outFile, err := os.Create(destPath)
				if err == nil {
					_, _ = io.Copy(outFile, rc)
					outFile.Close()
					imgFiles = append(imgFiles, destPath)
				}
				rc.Close()
			}
		}
	}

	if len(imgFiles) == 0 {
		return errors.New("no image pages found in archive")
	}

	naturalSort(imgFiles)
	for i, fPath := range imgFiles {
		pNum := i + 1
		sess.PageFiles[pNum] = fPath
		sess.Pages = append(sess.Pages, reading.Page{
			PageNumber: int32(pNum),
			URL:        fmt.Sprintf("/api/reading/local/file?id=%s&page=%d", sess.ID, pNum),
		})
	}
	sess.TotalPages = len(imgFiles)
	return nil
}

func (s *LocalReaderService) extractTarArchive(srcTar, destDir string, sess *LocalBookSession) error {
	f, err := os.Open(srcTar)
	if err != nil {
		return err
	}
	defer f.Close()

	var tr *tar.Reader
	if strings.HasSuffix(srcTar, ".gz") || strings.HasSuffix(srcTar, ".tgz") {
		gzr, err := gzip.NewReader(f)
		if err != nil {
			return err
		}
		defer gzr.Close()
		tr = tar.NewReader(gzr)
	} else {
		tr = tar.NewReader(f)
	}

	var imgFiles []string
	for {
		hdr, err := tr.Next()
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}
		if !hdr.FileInfo().IsDir() && isImageFile(hdr.Name) {
			destPath := filepath.Join(destDir, filepath.Base(hdr.Name))
			outFile, err := os.Create(destPath)
			if err == nil {
				_, _ = io.Copy(outFile, tr)
				outFile.Close()
				imgFiles = append(imgFiles, destPath)
			}
		}
	}

	if len(imgFiles) == 0 {
		return errors.New("no image pages found in tar archive")
	}

	naturalSort(imgFiles)
	for i, fPath := range imgFiles {
		pNum := i + 1
		sess.PageFiles[pNum] = fPath
		sess.Pages = append(sess.Pages, reading.Page{
			PageNumber: int32(pNum),
			URL:        fmt.Sprintf("/api/reading/local/file?id=%s&page=%d", sess.ID, pNum),
		})
	}
	sess.TotalPages = len(imgFiles)
	return nil
}

func (s *LocalReaderService) extractExternalArchive(srcArchive, destDir string, sess *LocalBookSession) error {
	var cmd *exec.Cmd
	ext := strings.ToLower(filepath.Ext(srcArchive))
	if ext == ".7z" {
		cmd = exec.Command("7z", "e", "-y", fmt.Sprintf("-o%s", destDir), srcArchive)
	} else {
		cmd = exec.Command("unrar", "e", "-y", srcArchive, destDir)
		if _, err := exec.LookPath("unrar"); err != nil {
			cmd = exec.Command("7z", "e", "-y", fmt.Sprintf("-o%s", destDir), srcArchive)
		}
	}

	_ = cmd.Run()

	entries, err := os.ReadDir(destDir)
	if err != nil {
		return err
	}

	var imgFiles []string
	for _, e := range entries {
		if !e.IsDir() && isImageFile(e.Name()) {
			imgFiles = append(imgFiles, filepath.Join(destDir, e.Name()))
		}
	}

	if len(imgFiles) == 0 {
		return fmt.Errorf("could not extract comic archive %s (ensure 7z or unrar is installed for rar/7z formats)", filepath.Base(srcArchive))
	}

	naturalSort(imgFiles)
	for i, fPath := range imgFiles {
		pNum := i + 1
		sess.PageFiles[pNum] = fPath
		sess.Pages = append(sess.Pages, reading.Page{
			PageNumber: int32(pNum),
			URL:        fmt.Sprintf("/api/reading/local/file?id=%s&page=%d", sess.ID, pNum),
		})
	}
	sess.TotalPages = len(imgFiles)
	return nil
}

func extractDocxText(docxPath string) (string, error) {
	r, err := zip.OpenReader(docxPath)
	if err != nil {
		return "", err
	}
	defer r.Close()

	var docXML io.ReadCloser
	for _, f := range r.File {
		if f.Name == "word/document.xml" {
			docXML, err = f.Open()
			break
		}
	}
	if docXML == nil {
		return "", errors.New("word/document.xml not found in docx archive")
	}
	defer docXML.Close()

	data, err := io.ReadAll(docXML)
	if err != nil {
		return "", err
	}

	decoder := xml.NewDecoder(bytes.NewReader(data))
	var sb strings.Builder
	var currentP strings.Builder

	for {
		t, err := decoder.Token()
		if err == io.EOF {
			break
		}
		if err != nil {
			break
		}

		switch se := t.(type) {
		case xml.StartElement:
			if se.Name.Local == "p" {
				currentP.Reset()
			}
		case xml.EndElement:
			if se.Name.Local == "p" {
				text := strings.TrimSpace(currentP.String())
				if text != "" {
					if sb.Len() > 0 {
						sb.WriteString("\n\n")
					}
					sb.WriteString(text)
				}
				currentP.Reset()
			}
		case xml.CharData:
			currentP.WriteString(string(se))
		}
	}

	return sb.String(), nil
}

func isImageFile(filename string) bool {
	ext := strings.ToLower(filepath.Ext(filename))
	return ext == ".jpg" || ext == ".jpeg" || ext == ".png" || ext == ".webp" || ext == ".gif" || ext == ".avif"
}

func copyFile(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	out, err := os.Create(dst)
	if err != nil {
		return err
	}
	defer out.Close()

	_, err = io.Copy(out, in)
	return err
}

func naturalSort(files []string) {
	sort.Slice(files, func(i, j int) bool {
		return naturalLess(files[i], files[j])
	})
}

func naturalLess(s1, s2 string) bool {
	f1 := filepath.Base(s1)
	f2 := filepath.Base(s2)

	for len(f1) > 0 && len(f2) > 0 {
		isDigit1 := unicode.IsDigit(rune(f1[0]))
		isDigit2 := unicode.IsDigit(rune(f2[0]))

		if isDigit1 && isDigit2 {
			idx1 := 0
			for idx1 < len(f1) && unicode.IsDigit(rune(f1[idx1])) {
				idx1++
			}
			idx2 := 0
			for idx2 < len(f2) && unicode.IsDigit(rune(f2[idx2])) {
				idx2++
			}
			n1, _ := strconv.Atoi(f1[:idx1])
			n2, _ := strconv.Atoi(f2[:idx2])
			if n1 != n2 {
				return n1 < n2
			}
			f1 = f1[idx1:]
			f2 = f2[idx2:]
		} else {
			if f1[0] != f2[0] {
				return f1[0] < f2[0]
			}
			f1 = f1[1:]
			f2 = f2[1:]
		}
	}
	return len(f1) < len(f2)
}
