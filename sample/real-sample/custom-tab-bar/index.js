const app = getApp();

Component({
  data: {
    selected: 0,
    cartCount: 0
  },

  lifetimes: {
    attached: function() {
      this.updateSelected();
      this.updateCartCount();
    }
  },

  methods: {
    updateSelected: function() {
      const pages = getCurrentPages();
      const currentPage = pages[pages.length - 1];
      const route = currentPage.route;

      let selected = 0;
      if (route.includes('index')) selected = 0;
      else if (route.includes('category')) selected = 1;
      else if (route.includes('cart')) selected = 2;
      else if (route.includes('profile')) selected = 3;

      this.setData({ selected });
    },

    updateCartCount: function() {
      this.setData({ cartCount: app.globalData.cartCount });
    },

    onSwitchTab: function(e) {
      const index = e.currentTarget.dataset.index;
      const path = e.currentTarget.dataset.path;

      if (this.data.selected !== index) {
        wx.switchTab({ url: path });
      }
    }
  }
});
