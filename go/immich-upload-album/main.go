package main

import (
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/table"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/go-resty/resty/v2"
	"github.com/joho/godotenv"
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
}

type model struct {
	albums        []album
	albumList     list.Model
	assets        []asset
	showAssets    bool
	selectedAlbum string
	filter        string // "all", "photos", "videos"
	assetTable    table.Model
}

func loadEnvVariables() (string, string) {
	err := godotenv.Load()
	if err != nil {
		log.Fatalf("Error loading .env file")
	}

	apiURL := os.Getenv("API_URL")
	apiKey := os.Getenv("API_KEY")

	return apiURL, apiKey
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

func fetchAlbumInfo(apiURL, apiKey, albumID string) ([]asset, error) {
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
	return albumInfo.Assets, nil
}

func initialModel(apiURL, apiKey string) (model, error) {
	albums, err := fetchAlbums(apiURL, apiKey)
	if err != nil {
		return model{}, err
	}

	items := []list.Item{}
	for _, album := range albums {
		items = append(items, album)
	}

	const defaultWidth = 20
	l := list.New(items, list.NewDefaultDelegate(), defaultWidth, 14)
	l.Title = "Select an album"
	l.SetShowStatusBar(false)
	l.SetFilteringEnabled(false)
	l.SetShowHelp(false)
	l.SetShowPagination(false)

	return model{albums: albums, albumList: l, filter: "all"}, nil
}

func (m *model) Init() tea.Cmd {
	return nil
}

func (m *model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "enter":
			if !m.showAssets {
				selectedAlbum := m.albums[m.albumList.Index()]
				m.selectedAlbum = selectedAlbum.AlbumName
				assets, err := fetchAlbumInfo(os.Getenv("API_URL"), os.Getenv("API_KEY"), selectedAlbum.ID)
				if err != nil {
					return m, nil
				}
				m.assets = assets
				m.showAssets = true
				m.setupTable()
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
	}
	var cmd tea.Cmd
	if m.showAssets {
		m.assetTable, cmd = m.assetTable.Update(msg)
	} else {
		m.albumList, cmd = m.albumList.Update(msg)
	}
	return m, cmd
}

func (m *model) View() string {
	if m.showAssets {
		header := fmt.Sprintf("Assets in selected album (%s):\n\n", m.selectedAlbum)
		numAssets := len(m.filteredAssets())
		totalSize := fmt.Sprintf("\nNumber of assets: %d, Total size: %s\n", numAssets, formatSize(m.totalSize()))
		instructions := "\nPress 'q' to go back, 'p' to show photos, 'v' to show videos, 'a' to show all."
		return header + m.assetTable.View() + totalSize + instructions
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

func main() {
	apiURL, apiKey := loadEnvVariables()
	m, err := initialModel(apiURL, apiKey)
	if err != nil {
		log.Fatal(err)
	}
	p := tea.NewProgram(&m)
	if err := p.Start(); err != nil {
		log.Fatal(err)
	}
}
