#!/bin/bash

set -eu

rm -rf ~/.nuget/packages/sentry.cli/
rm -rf ~/.nuget/packages/sentry.cli.msbuild/
rm -f ./nupkg/*.nupkg
rm -rf ./config/
rm -rf **/bin/**
rm -rf **/obj/**

dotnet clean
# dotnet nuget locals all --clear
dotnet tool uninstall  sentry-cli || true

dotnet pack -c release -nologo Sentry.Cli.MSBuild/Sentry.Cli.MSBuild.csproj
dotnet pack -c release -nologo  Sentry.Cli/Sentry.Cli.csproj
dotnet new tool-manifest --force

dotnet tool install sentry.cli
#dotnet tool install --add-source ./nupkg sentry-cli

dotnet tool list

dotnet tool restore --no-cache

# If installed globally, becomes 'sentry-cli'
dotnet sentry-cli
