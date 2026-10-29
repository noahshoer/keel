import SwiftUI

/// Glass versions of web buttons, drawn exactly over the (hidden) web ones;
/// a tap is passed on to them: the drawer's theme switcher and settings
/// button, and the task sheet's X and •••.
struct PlacedButtons: View {
  @ObservedObject var model: CardModel
  let buttons: [DrawerButton]

  var body: some View {
    ZStack(alignment: .topLeading) {
      Color.clear
      ForEach(buttons, id: \.id) { b in
        let size = max(40, max(b.w, b.h))
        Button {
          model.button(b.id)
        } label: {
          Image(systemName: b.symbol)
            .font(.system(size: 18, weight: .medium))
            .foregroundStyle(Color.primary)
            .frame(width: size, height: size)
            .contentTransition(.symbolEffect(.replace))
        }
        .buttonStyle(.plain)
        .glassEffect(.regular.interactive(), in: .circle)
        .touchable("drawer-\(b.id)", in: model)
        .position(x: b.x + b.w / 2, y: b.y + b.h / 2)
        .accessibilityLabel(placedLabel(b.id))
        .transition(.opacity)
      }
    }
    .ignoresSafeArea()
  }
}

func placedLabel(_ id: String) -> String {
  switch id {
  case "theme": "Theme"
  default: "Settings"
  }
}

/// The floating + over the task list (bottom right). Hidden while the card,
/// the drawer or the task view is open. (The menu and search are glass inside
/// the page: `PageGlass`.)
struct FloatingButtons: View {
  @ObservedObject var model: CardModel

  var body: some View {
    VStack {
      Spacer()
      // The + steps aside while the card is open (the card covers it).
      if model.addVisible && !model.isOpen {
      HStack {
        Spacer()
        Button {
          model.button("fab")
        } label: {
          Image(systemName: "plus")
            .font(.title2.weight(.semibold))
            .foregroundStyle(.white)
            .frame(width: 58, height: 58)
        }
        .buttonStyle(.plain)
        .glassEffect(.regular.tint(keelGreen).interactive(), in: .circle)
        .touchable("fab", in: model)
        .accessibilityLabel("Add task")
      }
      .padding(.trailing, 20)
      .padding(.bottom, 12)
      }
    }
  }
}
