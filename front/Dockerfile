FROM node:slim as build

WORKDIR /app

COPY package.json .
RUN yarn

COPY . .

RUN yarn build

FROM nginx

COPY --from=build /app/build /usr/share/nginx/html