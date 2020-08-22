// A CGI script to render a simple gemini home page
package main

import (
  "database/sql"
  "bytes"
  "strings"
  "bufio"
  "html/template"
  "math/rand"
  "time"
  "fmt"
  _ "github.com/mattn/go-sqlite3"
)
// get config file

type File struct {
    UserName string
    FileName string
    UpdatedAt time.Time
}

func main() {
    database, _ := sql.Open("sqlite3", "../flounder.db")
    t, _ := template.ParseFiles("index.gmi")
    rows, _ := database.Query(`SELECT user.username, file.user_path, file.updated_at
FROM file
JOIN user
ON file.user_id = user.id
ORDER BY file.updated_at DESC
LIMIT 32`)
    var files []File
    for rows.Next() {
      var file File
      var unixTime int64
      _ = rows.Scan(&file.UserName, &file.FileName, &unixTime)
      file.UpdatedAt = time.Unix(unixTime, 0)
      files = append(files, file)
    }
    rows, _ = database.Query(`SELECT username from user`)
    var users []string
    for rows.Next() {
      var user string
      _ = rows.Scan(&user)
      users = append(users, user)
    }
    rand.Seed(time.Now().UnixNano())
    rand.Shuffle(len(users), func(i, j int) { users[i], users[j] = users[j], users[i] })


    data := struct {
      Domain string
      Files []File 
      Users []string
    }{
      Domain: "flounder.online",
      Files: files,
      Users: users,
    }
    buf := new(bytes.Buffer)
    _ = t.Execute(buf, data)
    newbuf := buf.String()
    scanner := bufio.NewScanner(strings.NewReader(newbuf))
    fmt.Print("20 text/gemini\r\n")
    for scanner.Scan() {
      fmt.Print(scanner.Text())
      fmt.Print("\r\n")
    }
// TODO -- output properly
    // todo err handling
}
