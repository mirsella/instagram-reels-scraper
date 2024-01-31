PREFIX=instagram-reels-scraper
NAME=emotion
all:
	docker rm $(NAME) || true
	docker build -t $(NAME) .
	docker create --name $(PREFIX)-$(NAME) $(PREFIX)-$(NAME)
