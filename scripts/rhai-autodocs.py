import sys

file = sys.argv[1]

oldfile = open(file, 'r')
newfile = open(f"{file}.autodocs", 'w')

function_order = 1
for line in oldfile:
    newline = ""
    if "# rhai-autodocs:index:" in line:
        newline = f"    /// # rhai-autodocs:index:{function_order}\n"
        function_order += 1
    else:
        newline = line

    newfile.write(newline)
