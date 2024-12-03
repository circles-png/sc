let executable = cargo build -r --message-format json | lines | last 2 | first | from json | get executable
print $executable
rm -r out
mkdir out/sc.app/Contents/MacOS/
cp $executable out/sc.app/Contents/MacOS/
cp src/Info.plist out/sc.app/
