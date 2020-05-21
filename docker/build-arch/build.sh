#! /bin/bash

docker build . -t git.thiesgerken.de:5005/thies/moneta/build-arch && docker push git.thiesgerken.de:5005/thies/moneta/build-arch
