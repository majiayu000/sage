# Read Tool - Demonstration Output

## Example 1: Basic File Reading

**Input:**
```json
{
  "file_path": "/path/to/file.txt"
}
```

**Output:**
```
     1→First line of the file
     2→Second line of the file
     3→Third line of the file
     4→Fourth line of the file
     5→Fifth line of the file
```

**Metadata:**
```json
{
  "file_path": "/path/to/file.txt",
  "total_lines": 5,
  "lines_read": 5,
  "start_line": 1,
  "end_line": 5,
  "truncated": false
}
```

---

## Example 2: Reading with Offset

**Input:**
```json
{
  "file_path": "/var/log/app.log",
  "offset": 100
}
```

**Output:**
```
   101→[2024-12-16 10:15:23] INFO: Application started
   102→[2024-12-16 10:15:24] DEBUG: Loading configuration
   103→[2024-12-16 10:15:25] INFO: Database connected
   104→[2024-12-16 10:15:26] WARN: High memory usage detected
```

**Metadata:**
```json
{
  "file_path": "/var/log/app.log",
  "total_lines": 1500,
  "lines_read": 1400,
  "start_line": 101,
  "end_line": 1500,
  "truncated": false
}
```

---

## Example 3: Reading with Limit (Pagination)

**Input:**
```json
{
  "file_path": "/home/user/data.csv",
  "limit": 10
}
```

**Output:**
```
     1→id,name,email,created_at
     2→1,John Doe,john@example.com,2024-01-01
     3→2,Jane Smith,jane@example.com,2024-01-02
     4→3,Bob Johnson,bob@example.com,2024-01-03
     5→4,Alice Williams,alice@example.com,2024-01-04
     6→5,Charlie Brown,charlie@example.com,2024-01-05
     7→6,Diana Prince,diana@example.com,2024-01-06
     8→7,Ethan Hunt,ethan@example.com,2024-01-07
     9→8,Fiona Green,fiona@example.com,2024-01-08
    10→9,George White,george@example.com,2024-01-09

[Content truncated: showing lines 1-10 of 1000 total lines. Use offset parameter to read more.]
```

**Metadata:**
```json
{
  "file_path": "/home/user/data.csv",
  "total_lines": 1000,
  "lines_read": 10,
  "start_line": 1,
  "end_line": 10,
  "truncated": true
}
```

---

## Example 4: Reading Specific Range

**Input:**
```json
{
  "file_path": "/path/to/source.rs",
  "offset": 49,
  "limit": 10
}
```

**Output:**
```
    50→    pub fn new() -> Self {
    51→        Self {
    52→            working_directory: std::env::current_dir()
    53→                .unwrap_or_else(|_| PathBuf::from(".")),
    54→        }
    55→    }
    56→
    57→    pub fn with_working_directory<P: Into<PathBuf>>(working_dir: P) -> Self {
    58→        Self {
    59→            working_directory: working_dir.into(),
```

**Metadata:**
```json
{
  "file_path": "/path/to/source.rs",
  "total_lines": 450,
  "lines_read": 10,
  "start_line": 50,
  "end_line": 59,
  "truncated": true
}
```

---

## Example 5: Long Line Truncation

**Input:**
```json
{
  "file_path": "/path/to/minified.js"
}
```

**Output:**
```
     1→(function(){var a=document.createElement('script');a.src='https://example.com/analytics.js';a.async=true;document.head.appendChild(a);var b=function(){console.log('Analytics loaded');};a.onload=b;var c={config:{apiKey:'1234567890abcdef',endpoint:'https://api.example.com/v1',timeout:5000,retries:3,debug:false,features:['tracking','analytics','reporting','monitoring','alerts','notifications','logging','metrics','dashboards','charts','graphs','tables','exports','imports','integrations','webhooks','apis','sdks','plugins','extensions','themes','templates','widgets','components','modules','packages','libraries','frameworks','tools','utilities','helpers','services','workers','jobs','tasks','queues','streams','events','listeners','handlers','callbacks','promises','async','await','fetch','xhr','websockets','sse','http','https','rest','graphql','soap','grpc','mqtt','amqp','kafka','redis','memcached','elasticsearch','mongodb','postgresql','mysql','sqlite','dynamodb','cassandra','couchbase','neo4j','influxdb','prometheus','grafana','kibana','logstash','fluentd','datadog','newrelic','sentry','rollbar','bugsnag','airbrake','honeybadger','raygun','appsignal','scout','skylight','blackfire','tideways','xhprof','kcachegrind','valgrind','perf','dtrace','strace','ltrace','gdb','lldb','windbg','ida','ghidra','radare2','hopper','binary ninja','immunity','ollydbg','x64dbg','windbg preview','visual studio','vscode','intellij','pycharm','webstorm','phpstorm','rubymine','clion','goland','rider','datagrip','appcode','android studio','xcode','eclipse','netbeans','atom','sublime','notepad++','vim','emacs','nano','vi','ed','sed','awk','grep','find','locate','which','whereis','file','stat','du','df','ls','cd','pwd','mkdir','rmdir','rm','cp','mv','ln','chmod','chown','chgrp','umask','touch','cat','less','more','head','tail','wc','sort','uniq','cut','paste','join','comm','diff','patch','tr','expand','unexpand','nl','od','hexdump','strings','xxd','base64','openssl','gpg','ssh','scp','rsync','curl','wget','telnet','ftp','sftp','nc','socat','nmap','tcpdump','wireshark','tshark','ettercap','burp','zap','metasploit','nessus','openvas','nikto','sqlmap','hydra','john','hashcat','aircrack','kismet','reaver','bettercap','mitmproxy','charles','fiddler','postman','insomnia','httpie','jq','yq','xml','json','yaml','toml','ini','csv','tsv','html','css','javascript','typescript','coffeescript','dart','go','rust','c','cpp','csharp','java','kotlin','scala','clojure','erlang','elixir','haskell','ocaml','fsharp','swift','objectivec','ruby','python','php','perl','lua','r','julia','matlab','octave','fortran','cobol','ada','pascal','delphi','vb','vba','powershell','bash','zsh','fish','ksh','tcsh','csh','sh']... [line truncated, 4567 chars total]
```

---

## Example 6: Binary File Detection (Image)

**Input:**
```json
{
  "file_path": "/path/to/logo.png"
}
```

**Output:**
```
[Image file detected: /path/to/logo.png]

This is a PNG image file. Binary content cannot be displayed as text.
File size: 45678 bytes
```

**Metadata:**
```json
{
  "file_path": "/path/to/logo.png",
  "total_lines": 0,
  "lines_read": 0,
  "start_line": 0,
  "end_line": 0,
  "truncated": false
}
```

---

## Example 7: Binary File Detection (PDF)

**Input:**
```json
{
  "file_path": "/path/to/document.pdf"
}
```

**Output:**
```
[PDF file detected: /path/to/document.pdf]

This is a PDF file. Binary content cannot be displayed as text.
File size: 234567 bytes

To extract text from PDF, consider using a dedicated PDF processing tool.
```

---

## Example 8: Empty File

**Input:**
```json
{
  "file_path": "/path/to/empty.txt"
}
```

**Output:**
```
(empty string)
```

**Metadata:**
```json
{
  "file_path": "/path/to/empty.txt",
  "total_lines": 0,
  "lines_read": 0,
  "start_line": 0,
  "end_line": 0,
  "truncated": false
}
```

---

## Example 9: Error - File Not Found

**Input:**
```json
{
  "file_path": "/path/to/nonexistent.txt"
}
```

**Error:**
```
ToolError: ExecutionFailed("File not found: /path/to/nonexistent.txt")
```

---

## Example 10: Error - Directory

**Input:**
```json
{
  "file_path": "/path/to/directory"
}
```

**Error:**
```
ToolError: ExecutionFailed("Path is a directory, not a file: /path/to/directory")
```

---

## Example 11: Error - Invalid Offset

**Input:**
```json
{
  "file_path": "/path/to/small.txt",
  "offset": 1000
}
```

**Error:**
```
ToolError: InvalidArguments("Offset 1000 exceeds total lines 10 in file")
```

---

## Performance Characteristics

- **Small files (< 1KB)**: < 1ms
- **Medium files (1-100KB)**: 1-10ms
- **Large files (1-10MB)**: 10-100ms
- **Very large files (10-100MB)**: 100-1000ms

Memory usage is proportional to file size since the entire file is loaded into memory.
