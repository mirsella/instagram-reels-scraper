PREFIX=instagram-reels-scraper
NAME=emotion
all:
	docker rm $(PREFIX)-$(NAME) || true
	docker build -t $(PREFIX)-$(NAME) .
	docker create --name $(PREFIX)-$(NAME) $(PREFIX)-$(NAME)
