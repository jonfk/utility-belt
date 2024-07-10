package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/go-resty/resty/v2"
	gphotos "github.com/gphotosuploader/google-photos-api-client-go/v3"
	"github.com/joho/godotenv"
	"github.com/spf13/cobra"
	"golang.org/x/oauth2"
)

type album struct {
	AlbumName  string `json:"albumName"`
	AssetCount int    `json:"assetCount"`
	ID         string `json:"id"`
}

type asset struct {
	OriginalPath string `json:"originalPath"`
	Type         string `json:"type"` // Possible values: ["IMAGE", "VIDEO", "AUDIO", "OTHER"]
	ExifInfo     struct {
		FileSizeInByte int64 `json:"fileSizeInByte"`
	} `json:"exifInfo"`
	RealFilePath string
}

type EnvVariables struct {
	APIURL             string
	APIKey             string
	ContainerMountPath string
	RealPath           string
	ClientID           string
	ClientSecret       string
}

var envVars EnvVariables

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

func main() {
	envVars = loadEnvVariables()

	var rootCmd = &cobra.Command{Use: "photos-cli"}

	var listAlbumsCmd = &cobra.Command{
		Use:   "list-albums",
		Short: "List all albums",
		Run: func(cmd *cobra.Command, args []string) {
			albums, err := fetchAlbums(envVars.APIURL, envVars.APIKey)
			if err != nil {
				log.Fatal(err)
			}
			for _, album := range albums {
				fmt.Printf("%s - %s (%d assets)\n", album.ID, album.AlbumName, album.AssetCount)
			}
		},
	}

	var listAssetsCmd = &cobra.Command{
		Use:   "list-assets [albumID]",
		Short: "List all assets in an album",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			albumID := args[0]
			assets, err := fetchAlbumInfo(envVars.APIURL, envVars.APIKey, albumID, envVars.ContainerMountPath, envVars.RealPath)
			if err != nil {
				log.Fatal(err)
			}
			for _, asset := range assets {
				fmt.Printf("%s (%s)\n", filepath.Base(asset.OriginalPath), formatSize(asset.ExifInfo.FileSizeInByte))
			}
		},
	}

	var uploadCmd = &cobra.Command{
		Use:   "upload [albumName]",
		Short: "Upload assets to Google Photos",
		Args:  cobra.ExactArgs(1),
		Run: func(cmd *cobra.Command, args []string) {
			albumName := args[0]

			var authCode string
			fmt.Print("Enter authorization code: ")
			fmt.Scanln(&authCode)
			token := exchangeAuthCodeForToken(authCode)
			uploadAssets(albumName, token)
		},
	}

	rootCmd.AddCommand(listAlbumsCmd)
	rootCmd.AddCommand(listAssetsCmd)
	rootCmd.AddCommand(uploadCmd)
	if err := rootCmd.Execute(); err != nil {
		log.Fatal(err)
	}
}

// TODO fix google oauth2 authorization

func exchangeAuthCodeForToken(authCode string) *oauth2.Token {
	ctx := context.Background()

	oauth2Config := oauth2.Config{
		ClientID:     envVars.ClientID,
		ClientSecret: envVars.ClientSecret,
		RedirectURL:  "urn:ietf:wg:oauth:2.0:oob",
		Scopes:       []string{"https://www.googleapis.com/auth/photoslibrary"},
		Endpoint: oauth2.Endpoint{
			AuthURL:  "https://accounts.google.com/o/oauth2/auth",
			TokenURL: "https://accounts.google.com/o/oauth2/token",
		},
	}

	token, err := oauth2Config.Exchange(ctx, authCode)
	if err != nil {
		log.Fatalf("Error exchanging authorization code: %v", err)
	}
	return token
}

func uploadAssets(albumName string, token *oauth2.Token) {
	ctx := context.Background()

	tc := oauth2.NewClient(ctx, oauth2.StaticTokenSource(token))

	client, err := gphotos.NewClient(tc)
	if err != nil {
		log.Fatalf("Error creating Google Photos client: %v", err)
	}

	album, err := client.Albums.Create(ctx, albumName)
	if err != nil {
		log.Fatalf("Error creating album: %v", err)
	}

	assets, err := fetchAlbumInfo(envVars.APIURL, envVars.APIKey, album.ID, envVars.ContainerMountPath, envVars.RealPath)
	if err != nil {
		log.Fatalf("Error fetching album info: %v", err)
	}

	totalAssets := len(assets)
	for i, asset := range assets {
		uploadedMediaItem, err := client.UploadToAlbum(ctx, album.ID, asset.RealFilePath)

		if err != nil || uploadedMediaItem == nil {
			log.Fatalf("Error uploading media items: %v", err)
		}

		progress := float64(i+1) / float64(totalAssets)
		fmt.Printf("Upload progress: %.2f%%\n", progress*100)
	}
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
