#!/bin/bash

# Check if a language was passed.
if [ -z "$1" ]; then
  echo -e "\033[31;1m\xe2\x9c\x98 Error:\033[0m No language specified"
  echo "Usage: $0 <language>"
  exit 1
fi

# TODO: Fix this to work with the new Cognite library structure.

# Check if the language template exists.
if [ ! -d "handling/$1" ]
then
  echo -e "\033[31;1m\xe2\x9c\x98 Error:\033[0m Language '$1' is not supported"
  echo "Available languages: $(ls -m handling)"
  exit 1
fi

arrow="\033[34;1m~>\033[0m"

echo -ne "$arrow What's the name of your project? "
read name
echo -ne "\033[F\r   \n"

echo -e "$arrow Copying project files..."
cp -nr handling/$1/preset/* bot
echo -ne "\033[F\r   \n"

echo -e "$arrow Modifiying files..."
find bot -type f -exec sed -i -e "s/\\\$\\\$NAME\\\$\\\$/$name/g" {} \;
echo -ne "\033[F\r   \n"

if [ -f bot/setup.sh ]
then
  echo -e "$arrow Running preset setup script..."
  ./bot/setup.sh
  echo -ne "\033[F\r   \n"
  echo -e "$arrow Cleaning up..."
  rm bot/setup.sh
fi

echo -e "$arrow Initializing git repo..."
git init bot

echo -e "$arrow Done! Check the project in 'bot' directory"
echo "To start the bot run the init script in this directory."
