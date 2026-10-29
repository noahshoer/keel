import SwiftUI

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
