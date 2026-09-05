package ui

import "embed"

// DistFS contains the embedded static files for the Vessel Native Web/Desktop UI.
//
//go:embed index.html styles.css app.js
var DistFS embed.FS
