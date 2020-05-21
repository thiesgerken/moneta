#! /bin/bash

docker build . -t git.thiesgerken.de:5005/thies/moneta/build-ubuntu && docker push git.thiesgerken.de:5005/thies/moneta/build-ubuntu
