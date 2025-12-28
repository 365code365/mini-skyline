// 自定义 TabBar 组件
Component({
  data: {
    selected: 0,
    list: []
  },

  attached: function() {
    // 从 app.json 获取 tabBar 配置
    this.setData({
      list: getApp().globalData.tabBar.list
    });
  },

  methods: {
    switchTab: function(e) {
      var index = e.currentTarget.dataset.index;
      var path = e.currentTarget.dataset.path;
      
      this.setData({
        selected: index
      });
      
      wx.switchTab({
        url: '/' + path
      });
    }
  }
});
