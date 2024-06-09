package main

/*
High-Level Features of the Program

    Album and Asset Management via Terminal UI:
        Album Listing and Viewing: The program interacts with an external API (an Immich instance) to fetch and display a list of photo albums. Users can navigate and select albums to view the assets (photos and videos) they contain.
        Asset Filtering: Users can filter the displayed assets by type (photos or videos) or view all assets in the selected album.

    Environment Variable Configuration:
        The program loads configuration details such as API URLs, API keys, file paths, and OAuth credentials from environment variables to ensure it runs with the correct settings.

    Path Management:
        The program ensures each asset's path is correctly mapped from a container path to a real file system path, enabling it to locate and manage assets on the local file system accurately.

    Google Photos Integration:
        OAuth2 Authentication: The program authenticates with the Google Photos API using OAuth2.
        Album Creation in Google Photos: Users can create a new album in Google Photos by entering an album name.
        Asset Upload to Google Photos: The program uploads selected assets from the local file system to the newly created Google Photos album.

    Progress Tracking:
        The program tracks and displays the progress of the upload process to Google Photos, providing users with visual feedback on the operation's status.

How the Program Achieves These Features

    Terminal-Based User Interface:
        Uses a terminal-based UI framework to create a responsive and interactive experience where users can navigate lists, select items, and initiate actions such as viewing assets and starting uploads.

    Environment Configuration:
        Loads environment variables to configure the application, encapsulating these details in a structured format for easy access and management throughout the program.

    Path Management:
        Processes each asset's path to ensure it starts with the expected container mount path and replaces this prefix with the real file system path, allowing accurate location of assets on the local system.

    Google Photos Integration:
        Sets up OAuth2 configuration using client ID and secret, handling token management for authenticated API requests.
        Interacts with the Google Photos API to create albums and upload media items, ensuring proper error handling and reporting.

    Progress Tracking:
        Integrates a progress bar to visually indicate the status of the upload process, updating in real-time to reflect progress.
*/

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/progress"
	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/go-resty/resty/v2"
	gphotos "github.com/gphotosuploader/google-photos-api-client-go/v3"
	"github.com/joho/godotenv"
	"golang.org/x/oauth2"
)

type album struct {
	AlbumName  string `json:"albumName"`
	AssetCount int    `json:"assetCount"`
	ID         string `json:"id"`
}

func (a album) Title() string       { return a.AlbumName }
func (a album) Description() string { return fmt.Sprintf("%d assets", a.AssetCount) }
func (a album) FilterValue() string { return a.AlbumName }

type asset struct {
	OriginalPath string `json:"originalPath"`
	Type         string `json:"type"` // Possible values: ["IMAGE", "VIDEO", "AUDIO", "OTHER"]
	ExifInfo     struct {
		FileSizeInByte int64 `json:"fileSizeInByte"`
	} `json:"exifInfo"`
	RealFilePath string
}

type model struct {
	albums         []album
	albumList      list.Model
	assets         []asset
	showAssets     bool
	selectedAlbum  string
	filter         string // "all", "photos", "videos"
	assetTable     table.Model
	progress       progress.Model
	textInput      textinput.Model
	showTextInput  bool
	uploadProgress float64
	envVars        EnvVariables
}

type EnvVariables struct {
	APIURL             string
	APIKey             string
	ContainerMountPath string
	RealPath           string
	ClientID           string
	ClientSecret       string
}

func loadEnvVariables() EnvVariables {
	err := godotenv.Load()
	if err != nil {
		log.Fatalf("Error loading .env file")
	}

	return EnvVariables{
		APIURL:             os.Getenv("API_URL"),
		APIKey:             os.Getenv("API_KEY"),
		ContainerMountPath: os.Getenv("CONTAINER_MOUNT_PATH"),
		RealPath:           os.Getenv("REAL_PATH"),
		ClientID:           os.Getenv("CLIENT_ID"),
		ClientSecret:       os.Getenv("CLIENT_SECRET"),
	}
}

func fetchAlbums(apiURL, apiKey string) ([]album, error) {
	client := resty.New()
	resp, err := client.R().
		SetHeader("Accept", "application/json").
		SetHeader("x-api-key", apiKey).
		SetResult(&[]album{}).
		Get(apiURL + "/album")

	if err != nil {
		return nil, err
	}

	albums := *resp.Result().(*[]album)
	return albums, nil
}

func fetchAlbumInfo(apiURL, apiKey, albumID, containerMountPath, realPath string) ([]asset, error) {
	client := resty.New()
	resp, err := client.R().
		SetHeader("Accept", "application/json").
		SetHeader("x-api-key", apiKey).
		SetResult(&struct {
			Assets []asset `json:"assets"`
		}{}).
		Get(apiURL + "/album/" + albumID)

	if err != nil {
		return nil, err
	}

	albumInfo := resp.Result().(*struct {
		Assets []asset `json:"assets"`
	})
	assets := albumInfo.Assets
	for i, a := range assets {
		if !strings.HasPrefix(a.OriginalPath, containerMountPath) {
			return nil, errors.New(fmt.Sprintf("path %s does not start with %s", a.OriginalPath, containerMountPath))
		}
		assets[i].RealFilePath = strings.Replace(a.OriginalPath, containerMountPath, realPath, 1)
	}
	return assets, nil
}

func initialModel(envVars EnvVariables) (model, error) {
	albums, err := fetchAlbums(envVars.APIURL, envVars.APIKey)
	if err != nil {
		return model{}, err
	}

	items := []list.Item{}
	for _, album := range albums {
		items = append(items, album)
	}

	const defaultWidth = 0
	const defaultHeight = 0
	l := list.New(items, list.NewDefaultDelegate(), defaultWidth, defaultHeight)
	l.Title = "Select an album"

	p := progress.New(progress.WithScaledGradient("#FF7CCB", "#FDFF8C"))

	ti := textinput.New()
	ti.Placeholder = "Enter album name"
	ti.Focus()

	return model{
		albums:        albums,
		albumList:     l,
		filter:        "all",
		progress:      p,
		textInput:     ti,
		showTextInput: false,
		envVars:       envVars,
	}, nil
}

func (m *model) Init() tea.Cmd {
	return nil
}

func (m *model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "enter":
			if m.showTextInput {
				m.showTextInput = false
				go m.uploadAssets(m.textInput.Value())
				return m, nil
			}
			if !m.showAssets {
				selectedAlbum := m.albums[m.albumList.Index()]
				m.selectedAlbum = selectedAlbum.AlbumName
				assets, err := fetchAlbumInfo(m.envVars.APIURL, m.envVars.APIKey, selectedAlbum.ID, m.envVars.ContainerMountPath, m.envVars.RealPath)
				if err != nil {
					log.Fatal(err)
					return m, nil
				}
				m.assets = assets
				m.showAssets = true
				m.setupTable()
			} else {
				m.showTextInput = true
			}
		case "p":
			if m.showAssets {
				m.filter = "photos"
				m.setupTable()
			}
		case "v":
			if m.showAssets {
				m.filter = "videos"
				m.setupTable()
			}
		case "a":
			if m.showAssets {
				m.filter = "all"
				m.setupTable()
			}
		case "q":
			if m.showAssets {
				m.showAssets = false
				m.filter = "all"
			} else {
				return m, tea.Quit
			}
		}
	case tea.WindowSizeMsg:
		m.albumList.SetSize(msg.Width, msg.Height)
		m.assetTable.SetWidth(msg.Width)
		m.assetTable.SetHeight(msg.Height - 5) // Adjust height for header and footer
	case progress.FrameMsg:
		progressModel, cmd := m.progress.Update(msg)
		m.progress = progressModel.(progress.Model)
		return m, cmd
	}

	var cmd tea.Cmd
	if m.showAssets {
		m.assetTable, cmd = m.assetTable.Update(msg)
	} else {
		m.albumList, cmd = m.albumList.Update(msg)
	}

	if m.showTextInput {
		m.textInput, cmd = m.textInput.Update(msg)
		return m, cmd
	}

	return m, cmd
}

func (m *model) View() string {
	if m.showTextInput {
		return "\nEnter the name for the new Google Photos album:\n" + m.textInput.View() + "\n"
	}
	if m.showAssets {
		header := fmt.Sprintf("Assets in selected album (%s):\n\n", m.selectedAlbum)
		numAssets := len(m.filteredAssets())
		totalSize := fmt.Sprintf("\nNumber of assets: %d, Total size: %s\n", numAssets, formatSize(m.totalSize()))
		instructions := "\nPress 'q' to go back, 'p' to show photos, 'v' to show videos, 'a' to show all, 'enter' to upload assets."
		return header + m.assetTable.View() + totalSize + instructions + "\n" + m.progress.View()
	}
	return "\n" + m.albumList.View()
}

func (m *model) setupTable() {
	columns := []table.Column{
		{Title: "Filename", Width: 30},
		{Title: "Size", Width: 10},
		{Title: "Path", Width: 50},
	}

	rows := []table.Row{}
	for _, asset := range m.filteredAssets() {
		filename := filepath.Base(asset.OriginalPath)
		size := formatSize(asset.ExifInfo.FileSizeInByte)
		row := table.Row{filename, size, asset.OriginalPath}
		rows = append(rows, row)
	}

	m.assetTable = table.New(table.WithColumns(columns), table.WithRows(rows), table.WithFocused(true))
	m.assetTable.SetStyles(table.DefaultStyles())
}

func (m *model) filteredAssets() []asset {
	if m.filter == "all" {
		return m.assets
	}
	filtered := []asset{}
	for _, asset := range m.assets {
		if (m.filter == "photos" && asset.Type == "IMAGE") || (m.filter == "videos" && asset.Type == "VIDEO") {
			filtered = append(filtered, asset)
		}
	}
	return filtered
}

func (m *model) totalSize() int64 {
	var totalSize int64
	for _, asset := range m.filteredAssets() {
		totalSize += asset.ExifInfo.FileSizeInByte
	}
	return totalSize
}

func formatSize(size int64) string {
	const unit = 1024
	if size < unit {
		return fmt.Sprintf("%d B", size)
	}
	div, exp := int64(unit), 0
	for n := size / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	return fmt.Sprintf("%.1f %cB", float64(size)/float64(div), "KMGTPE"[exp])
}

func (m *model) uploadAssets(albumName string) {
	ctx := context.Background()

	oauth2Config := oauth2.Config{
		ClientID:     m.envVars.ClientID,
		ClientSecret: m.envVars.ClientSecret,
		RedirectURL:  "urn:ietf:wg:oauth:2.0:oob", // Use for out-of-band authentication
		Scopes:       []string{"https://www.googleapis.com/auth/photoslibrary"},
		Endpoint: oauth2.Endpoint{
			AuthURL:  "https://accounts.google.com/o/oauth2/auth",
			TokenURL: "https://accounts.google.com/o/oauth2/token",
		},
	}

	// Generate URL for the user to visit
	authURL := oauth2Config.AuthCodeURL("state-token", oauth2.AccessTypeOffline)
	fmt.Printf("Visit the URL for the auth dialog: %v\n", authURL)

	// Read the authorization code
	var authCode string
	fmt.Printf("Enter the authorization code: ")
	if _, err := fmt.Scan(&authCode); err != nil {
		log.Fatalf("Error reading authorization code: %v", err)
	}

	// Exchange authorization code for an access token
	token, err := oauth2Config.Exchange(ctx, authCode)
	if err != nil {
		log.Fatalf("Error exchanging authorization code: %v", err)
	}

	// Create an authenticated HTTP client
	tc := oauth2Config.Client(ctx, token)

	client, err := gphotos.NewClient(tc)
	if err != nil {
		log.Fatalf("Error creating Google Photos client: %v", err)
	}

	album, err := client.Albums.Create(ctx, albumName)
	if err != nil {
		log.Fatalf("Error creating album: %v", err)
	}

	for _, asset := range m.assets {
		uploadedMediaItem, err := client.UploadToAlbum(ctx, album.ID, asset.RealFilePath)

		if err != nil || uploadedMediaItem == nil {
			log.Fatalf("Error uploading media items: %v", err)
		}
	}

	m.uploadProgress = 1.0
}

func main() {
	envVars := loadEnvVariables()
	m, err := initialModel(envVars)
	if err != nil {
		log.Fatal(err)
	}
	p := tea.NewProgram(&m)
	if err := p.Start(); err != nil {
		log.Fatal(err)
	}
}
