#!/bin/bash

# Check if a command is provided
if [ $# -eq 0 ]; then
    echo "Usage: $0 -- <command>"
    exit 1
fi

# Remove the -- separator
shift

# Join the remaining arguments into a single command
command="$*"

# Create a temporary file for the Vim script
temp_vim_script=$(mktemp)

# Write the Vim script to the temporary file
cat << EOF > "$temp_vim_script"
" Get the command from the first argument
let g:pipe_command = get(g:, 'pipe_command', 'cat')

function! SetupSpecialBuffer()
    " Create a new buffer
    enew
    
    " Configure the buffer
    setlocal buftype=acwrite
    setlocal bufhidden=hide
    setlocal noswapfile
    
    " Set up custom write behavior
    autocmd BufWriteCmd <buffer> call SaveSpecialBuffer()
    
    " Set buffer name
    file SpecialBuffer
endfunction

function! SaveSpecialBuffer()
    " Get buffer contents
    let l:contents = join(getline(1, '$'), "\n")
    
    " Pipe contents through the command
    let l:output = system(g:pipe_command, l:contents)
    
    " Display output (you might want to adjust this based on your needs)
    echo l:output
    
    " Trigger BufWritePost
    silent doautocmd BufWritePost
    
    setlocal nomodified
    return 0  " Indicate successful save
endfunction

" Set up the buffer when Vim starts
autocmd VimEnter * call SetupSpecialBuffer()
EOF

# Launch Neovim with the special configuration
nvim -c "let g:pipe_command='$command'" -S "$temp_vim_script"

# Clean up the temporary file
rm "$temp_vim_script"
