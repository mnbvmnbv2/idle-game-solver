import sys

from game import Game

from PyQt5.QtWidgets import (
    QApplication,
    QLabel,
    QHBoxLayout,
    QPushButton,
    QVBoxLayout,
    QWidget,
)


class MainWidget(QWidget):
    def __init__(self, goal):
        super().__init__()

        self.game = Game()
        self.goal = goal
        self.init_ui()

    def init_ui(self):
        # Main
        main_layout = QVBoxLayout()
        self.money_label = QLabel(f"Money: {self.game.money:.2f}")
        explain_label = QLabel("(Quantity, Price, Income)")
        main_layout.addWidget(self.money_label)
        main_layout.addWidget(explain_label)

        # Resources
        resources_layout = QHBoxLayout()
        self.buttons = []
        for idx, r in enumerate(self.game.resources):
            self.buttons.append(QPushButton(str(r)))
            resources_layout.addWidget(self.buttons[idx])
        main_layout.addLayout(resources_layout)

        # Iterate button
        self.next_button = QPushButton("Next")
        main_layout.addWidget(self.next_button)

        self.setLayout(main_layout)

        # Logic
        for idx, button in enumerate(self.buttons):
            button.clicked.connect(lambda _, idx=idx: self.game.buy(idx))
            button.clicked.connect(self.update_texts)
        self.next_button.clicked.connect(self.next_step)

    def update_texts(self):
        for idx, button in enumerate(self.buttons):
            button.setText(str(self.game.resources[idx]))
        self.money_label.setText(f"Money: {self.game.money:.2f}")

    def next_step(self):
        self.game.step(self.goal, verbose=True)
        self.update_texts()


def main():
    app = QApplication(sys.argv)
    m = MainWidget(5000)
    m.show()
    sys.exit(app.exec_())


if __name__ == "__main__":
    main()
